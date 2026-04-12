-- 0044: shipment push attempts tracking
create table if not exists shipment_push_attempts (
  attempt_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  outbox_id uuid not null,
  shipment_id uuid not null,
  shop_id uuid not null references shops(shop_id),
  platform text not null,
  platform_order_id text not null,
  carrier_code text,
  tracking_no text,
  shipped_at timestamptz,
  status text not null,
  attempt int not null,
  duration_ms int not null,
  error_code text not null default '',
  error_message text not null default '',
  request_json jsonb not null default '{}'::jsonb,
  response_json jsonb not null default '{}'::jsonb,
  next_retry_at timestamptz,
  dead_letter_id uuid,
  trigger text not null default 'AUTO',
  created_at timestamptz not null default now()
);

create unique index if not exists idx_shipment_push_attempts_unique
  on shipment_push_attempts(tenant_id, outbox_id, attempt);
create index if not exists idx_shipment_push_attempts_tenant_status
  on shipment_push_attempts(tenant_id, status, created_at desc);
create index if not exists idx_shipment_push_attempts_shipment
  on shipment_push_attempts(tenant_id, shipment_id, created_at desc);
create index if not exists idx_shipment_push_attempts_dead_letter
  on shipment_push_attempts(tenant_id, dead_letter_id);
