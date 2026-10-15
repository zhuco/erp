-- 0117: inspection standards and readings. A missing required reading is not a pass.
-- Release names an inspector and does not default an open item to passed.

create table if not exists mfg_quality_items (
  tenant_id uuid not null references tenants(tenant_id),
  item_id uuid not null default gen_random_uuid(),
  work_order_id uuid not null,
  batch_code text not null,
  item_code text not null,
  required boolean not null,
  min_micros bigint not null,
  max_micros bigint not null,
  reading_micros bigint not null,
  reading_present boolean not null,
  passed boolean not null,
  inspector text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, item_id)
);

create table if not exists mfg_quality_releases (
  tenant_id uuid not null references tenants(tenant_id),
  release_id uuid not null default gen_random_uuid(),
  work_order_id uuid not null,
  receipt_id uuid not null,
  inspector text not null,
  result text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, release_id)
);
