-- 0109: production receipts. Reported output and received stock stay separate.
-- Held goods are not available for sale.

create table if not exists mfg_receipts (
  tenant_id uuid not null references tenants(tenant_id),
  receipt_id uuid not null default gen_random_uuid(),
  work_order_id uuid not null,
  qty_micros bigint not null,
  status text not null,
  available_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, receipt_id),
  constraint mfg_receipts_status_check
    check (status in ('HOLD', 'RELEASED'))
);
