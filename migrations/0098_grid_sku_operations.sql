-- SKU grid mutation identity and receipt are retained indefinitely so a delayed
-- retry cannot execute the same logical operation twice.
create table if not exists grid_sku_operations (
  tenant_id uuid not null references tenants(tenant_id),
  actor_user_id uuid not null,
  operation_id uuid not null,
  command jsonb not null,
  receipt jsonb,
  created_at timestamptz not null default now(),
  primary key (tenant_id, actor_user_id, operation_id)
);
