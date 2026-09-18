-- Frozen, actor-bound product-grid selections for bounded all-matching actions.
create table if not exists grid_product_selections (
  token uuid primary key,
  tenant_id uuid not null references tenants(tenant_id),
  actor_user_id uuid not null,
  normalized_query jsonb not null,
  query_fingerprint text not null,
  total bigint not null check (total between 0 and 100000),
  expires_at timestamptz not null,
  created_at timestamptz not null default now()
);

create index if not exists idx_grid_product_selections_actor_expiry
  on grid_product_selections(tenant_id, actor_user_id, expires_at);

create index if not exists idx_grid_product_selections_expiry
  on grid_product_selections(expires_at);

create table if not exists grid_product_selection_items (
  selection_token uuid not null references grid_product_selections(token) on delete cascade,
  ordinal bigint not null check (ordinal >= 0),
  product_id uuid not null,
  product_version timestamptz not null,
  data jsonb not null,
  primary key (selection_token, ordinal),
  unique (selection_token, product_id)
);
