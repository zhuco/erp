-- A search condition stays typed. An invalid range does not start a query.
-- A named view belongs to one enterprise, one user, and one page.

create table if not exists search_conditions (
  tenant_id uuid not null,
  condition_id uuid not null,
  user_id text not null,
  page_code text not null,
  kind_code text not null,
  join_code text not null,
  field_name text not null,
  low_text text not null,
  high_text text not null,
  decision text not null,
  generation_no bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, condition_id)
);

create table if not exists named_search_views (
  tenant_id uuid not null,
  view_id uuid not null,
  user_id text not null,
  page_code text not null,
  view_name text not null,
  field_name text not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, view_id),
  unique (tenant_id, user_id, page_code, view_name)
);

create table if not exists search_resets (
  tenant_id uuid not null,
  reset_id uuid not null,
  page_code text not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, reset_id)
);
