-- Resource meters stay with one enterprise and one task window.
-- A quota is not a feature entitlement. A replay must not charge twice.

create table if not exists usage_meters (
  tenant_id uuid not null,
  meter_id uuid not null,
  task_id text not null,
  resource_kind text not null,
  quantity bigint not null,
  window_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, meter_id),
  unique (tenant_id, task_id, resource_kind, window_text)
);

create table if not exists usage_quotas (
  tenant_id uuid not null,
  quota_id uuid not null,
  resource_kind text not null,
  quota_qty bigint not null,
  task_cap integer not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, quota_id),
  unique (tenant_id, resource_kind)
);

create table if not exists usage_notices (
  tenant_id uuid not null,
  notice_id uuid not null,
  resource_kind text not null,
  decision text not null,
  expand_option text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, notice_id)
);
