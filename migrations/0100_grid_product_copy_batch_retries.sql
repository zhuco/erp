-- Idempotent identities for server-side review snapshots of rejected copy items.
-- Rows are retained after the short-lived selection expires, so a lost response
-- never causes the same request identity to mint a different snapshot.
create table if not exists grid_product_copy_batch_retry_requests (
  tenant_id uuid not null references tenants(tenant_id),
  actor_user_id uuid not null,
  request_id uuid not null,
  original_batch_id uuid not null,
  selection_token uuid not null,
  query_fingerprint text not null,
  total bigint not null check (total between 1 and 100000),
  expires_at timestamptz not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, actor_user_id, request_id),
  foreign key (tenant_id, actor_user_id, original_batch_id)
    references grid_product_copy_batches(tenant_id, actor_user_id, batch_id)
);

create index if not exists idx_grid_product_copy_batch_retry_original
  on grid_product_copy_batch_retry_requests(
    tenant_id,
    actor_user_id,
    original_batch_id,
    created_at desc
  );
