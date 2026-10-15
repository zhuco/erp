-- 0166: frozen inspection templates. A later template edit does not rewrite the freeze.
-- Required items that are unread or failed block issue, available receipt, and shipment.

create table if not exists shared_inspection_templates (
  tenant_id uuid not null references tenants(tenant_id),
  template_id uuid not null default gen_random_uuid(),
  template_code text not null,
  version_no integer not null,
  kind text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, template_id),
  unique (tenant_id, template_code, version_no)
);

create table if not exists shared_inspection_template_lines (
  tenant_id uuid not null references tenants(tenant_id),
  template_id uuid not null,
  item_code text not null,
  required boolean not null,
  min_micros bigint not null,
  max_micros bigint not null,
  sample_count bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, template_id, item_code)
);

create table if not exists shared_inspection_freezes (
  tenant_id uuid not null references tenants(tenant_id),
  freeze_id uuid not null default gen_random_uuid(),
  template_id uuid not null,
  version_no integer not null,
  source_kind text not null,
  source_id uuid not null,
  item_code text not null,
  batch_code text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, freeze_id)
);

alter table mfg_quality_items add column if not exists freeze_id uuid;
alter table mfg_quality_items add column if not exists source_kind text not null default '';
alter table mfg_quality_items add column if not exists source_id uuid;
alter table mfg_quality_items alter column work_order_id drop not null;

alter table shared_sales_deliveries add column if not exists item_code text not null default '';

create table if not exists shared_inspection_freeze_lines (
  tenant_id uuid not null references tenants(tenant_id),
  freeze_id uuid not null,
  item_code text not null,
  required boolean not null,
  min_micros bigint not null,
  max_micros bigint not null,
  sample_count bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, freeze_id, item_code)
);
