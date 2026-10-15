-- 0106: fractional manufacturing material ledger.
-- Quantities are millionths of a base unit. This does not change legacy integer inventory columns.

create table if not exists mfg_qty_ledgers (
  tenant_id uuid not null references tenants(tenant_id),
  ledger_id uuid not null default gen_random_uuid(),
  warehouse_id uuid not null,
  sku_id uuid not null,
  move_id uuid not null,
  delta_micros bigint not null,
  reason_type text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, ledger_id),
  unique (tenant_id, move_id)
);
