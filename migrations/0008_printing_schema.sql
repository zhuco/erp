-- 0008: Printing devices + templates + jobs

create table if not exists print_devices (
  device_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  device_fingerprint text not null,
  operator_user_id uuid references users(user_id),
  default_printer_name text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (device_fingerprint)
);

create table if not exists print_templates (
  template_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shop_id uuid not null references shops(shop_id),
  platform text not null,
  template_type text not null,
  platform_template_id text,
  version int not null default 1,
  content jsonb not null default '{}'::jsonb,
  is_active boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, shop_id, template_type, version)
);

create table if not exists print_jobs (
  job_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shop_id uuid not null references shops(shop_id),
  type text not null,
  ref_type text not null,
  ref_id uuid not null,
  payload jsonb not null default '{}'::jsonb,
  status text not null default 'QUEUED',
  target_printer_name text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  printed_at timestamptz
);

create index if not exists idx_print_jobs_status_time
  on print_jobs(tenant_id, status, created_at);
create index if not exists idx_print_jobs_ref
  on print_jobs(tenant_id, ref_type, ref_id);
