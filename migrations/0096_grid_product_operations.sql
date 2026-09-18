-- Owned grid actions: receipt and mutation commit atomically. No automatic TTL:
-- deleting an operation identity would permit a delayed retry to execute again.
create table if not exists grid_product_operations (
  tenant_id uuid not null references tenants(tenant_id),
  actor_user_id uuid not null,
  operation_id uuid not null,
  command jsonb not null,
  receipt jsonb,
  created_at timestamptz not null default now(),
  primary key (tenant_id, actor_user_id, operation_id)
);
