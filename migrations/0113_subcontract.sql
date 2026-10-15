-- 0113: subcontract location is the supplier. Title stays with our work order.
-- A receipt does not consume issued material a second time.

create table if not exists mfg_subcontract_orders (
  tenant_id uuid not null references tenants(tenant_id),
  subcontract_id uuid not null default gen_random_uuid(),
  work_order_id uuid not null,
  supplier_id uuid not null,
  component_code text not null,
  our_material boolean not null,
  supplier_location text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, subcontract_id)
);

create table if not exists mfg_subcontract_movements (
  tenant_id uuid not null references tenants(tenant_id),
  move_id uuid not null default gen_random_uuid(),
  subcontract_id uuid not null,
  work_order_id uuid not null,
  kind text not null,
  qty_micros bigint not null,
  fee_cent bigint not null default 0,
  inspection_result text,
  period_key text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, move_id)
);

alter table mfg_subcontract_movements add column if not exists period_key text not null default '';
