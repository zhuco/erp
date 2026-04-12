-- 0083: WMS scan rules, QC sampling, inventory freeze approvals

create table if not exists wms_scan_rules (
  rule_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  scenario text not null,
  scan_mode text not null,
  block_on_mismatch boolean not null default true,
  enabled boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, warehouse_id, scenario)
);

create index if not exists idx_wms_scan_rules_tenant_updated
  on wms_scan_rules(tenant_id, updated_at desc, rule_id desc);

create table if not exists wms_qc_sampling_rules (
  rule_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  doc_type text not null default 'ALL',
  sample_rate int not null default 100,
  enabled boolean not null default true,
  note text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, warehouse_id, doc_type)
);

create index if not exists idx_wms_qc_sampling_rules_tenant_updated
  on wms_qc_sampling_rules(tenant_id, updated_at desc, rule_id desc);

create table if not exists inventory_freeze_approvals (
  approval_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  action text not null,
  warehouse_id uuid not null references warehouses(warehouse_id),
  sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  reason text,
  freeze_id uuid references inventory_freezes(freeze_id),
  status text not null default 'PENDING',
  requested_by uuid,
  approved_by uuid,
  approved_at timestamptz,
  approval_note text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_inventory_freeze_approvals_tenant_status
  on inventory_freeze_approvals(tenant_id, status, created_at desc);
create index if not exists idx_inventory_freeze_approvals_tenant_action
  on inventory_freeze_approvals(tenant_id, action, created_at desc);
