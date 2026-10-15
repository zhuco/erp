-- Billing orders, callbacks, entitlements, and refunds stay in separate rows.
-- A browser claim of success does not mark an order paid.

create table if not exists billing_orders (
  tenant_id uuid not null,
  order_id uuid not null,
  order_no text not null,
  plan_code text not null,
  amount_cent bigint not null,
  currency text not null,
  payment_status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, order_id),
  unique (tenant_id, order_no)
);

create table if not exists billing_callbacks (
  tenant_id uuid not null,
  callback_id uuid not null,
  order_no text not null,
  event_id text not null,
  signed_amount bigint not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, callback_id),
  unique (tenant_id, event_id)
);

create table if not exists billing_entitlements (
  tenant_id uuid not null,
  entitlement_id uuid not null,
  order_no text not null,
  version_no integer not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, entitlement_id)
);

create table if not exists billing_refunds (
  tenant_id uuid not null,
  refund_id uuid not null,
  order_no text not null,
  amount_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, refund_id)
);
