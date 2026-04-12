-- 0084: 订单状态历史与动作日志

create table if not exists order_status_history (
  history_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid not null,
  from_status text,
  to_status text not null,
  action_code text,
  operator_id uuid,
  reason text,
  payload jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

create index if not exists idx_order_status_history_tenant_order
  on order_status_history(tenant_id, order_id, created_at desc, history_id desc);
create index if not exists idx_order_status_history_action
  on order_status_history(tenant_id, action_code, created_at desc, history_id desc);

create table if not exists order_action_log (
  action_log_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid not null,
  action_code text not null,
  idempotency_key text not null,
  status text not null default 'SUCCESS',
  operator_id uuid,
  request_json jsonb not null default '{}'::jsonb,
  response_json jsonb not null default '{}'::jsonb,
  error_message text not null default '',
  created_at timestamptz not null default now()
);

create unique index if not exists idx_order_action_log_idempotency
  on order_action_log(tenant_id, order_id, action_code, idempotency_key);
create index if not exists idx_order_action_log_tenant_order
  on order_action_log(tenant_id, order_id, created_at desc, action_log_id desc);
create index if not exists idx_order_action_log_action
  on order_action_log(tenant_id, action_code, created_at desc, action_log_id desc);
