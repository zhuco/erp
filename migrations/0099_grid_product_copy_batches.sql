-- Durable server-side execution of a frozen all-matching product copy.
-- Batch and item rows are retained; selection expiry cannot erase execution identity.
create table if not exists grid_product_copy_batches (
  tenant_id uuid not null references tenants(tenant_id),
  actor_user_id uuid not null,
  batch_id uuid not null,
  request jsonb not null,
  total bigint not null check (total between 0 and 100000),
  applied bigint not null default 0 check (applied >= 0),
  rejected bigint not null default 0 check (rejected >= 0),
  cancelled bigint not null default 0 check (cancelled >= 0),
  pending bigint not null default 0 check (pending >= 0),
  unknown bigint not null default 0 check (unknown >= 0),
  cancel_requested boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  completed_at timestamptz,
  primary key (tenant_id, actor_user_id, batch_id),
  check (applied + rejected + cancelled + pending + unknown = total)
);

create index if not exists idx_grid_product_copy_batches_active
  on grid_product_copy_batches(tenant_id, actor_user_id, created_at)
  where completed_at is null;

create table if not exists grid_product_copy_batch_items (
  tenant_id uuid not null,
  actor_user_id uuid not null,
  batch_id uuid not null,
  ordinal bigint not null check (ordinal >= 0),
  source_product_id uuid not null,
  operation_id uuid not null,
  command jsonb not null,
  state text not null default 'PENDING'
    check (state in ('PENDING','IN_FLIGHT','APPLIED','REJECTED','CANCELLED')),
  lease_id uuid,
  lease_until timestamptz,
  receipt jsonb,
  updated_at timestamptz not null default now(),
  primary key (tenant_id, actor_user_id, batch_id, ordinal),
  unique (tenant_id, actor_user_id, operation_id),
  foreign key (tenant_id, actor_user_id, batch_id)
    references grid_product_copy_batches(tenant_id, actor_user_id, batch_id)
);

create index if not exists idx_grid_product_copy_batch_items_claim
  on grid_product_copy_batch_items(tenant_id, actor_user_id, batch_id, state, ordinal);
