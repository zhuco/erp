-- 0011: API idempotency keys

create table if not exists idempotency_keys (
  tenant_id uuid not null,
  "key" text not null,
  route text not null,
  request_hash text not null,
  response_status int not null,
  response_body jsonb not null,
  created_at timestamptz not null default now(),
  unique (tenant_id, route, "key")
);
