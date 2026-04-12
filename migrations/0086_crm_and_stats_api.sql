-- 0086: CRM 扩展能力与统计 API 基础表

create table if not exists crm_customer_segments (
  segment_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  segment_type text not null,
  rule_summary text,
  customer_count bigint not null default 0,
  status text not null default 'ACTIVE',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_crm_customer_segments_tenant_status
  on crm_customer_segments(tenant_id, status, updated_at desc);

create table if not exists crm_lifecycle_tasks (
  task_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  stage text not null,
  strategy text not null,
  trigger_rule text,
  owner text,
  status text not null default 'ACTIVE',
  last_triggered_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_crm_lifecycle_tasks_tenant_status
  on crm_lifecycle_tasks(tenant_id, status, updated_at desc);

create table if not exists crm_service_tickets (
  ticket_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  customer_id text,
  customer_name text not null,
  contact_phone text,
  channel text,
  category text,
  priority text,
  status text not null default 'OPEN',
  related_order_id text,
  satisfaction_score int,
  handler text,
  description text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_crm_service_tickets_tenant_status
  on crm_service_tickets(tenant_id, status, updated_at desc);

create index if not exists idx_crm_service_tickets_tenant_customer
  on crm_service_tickets(tenant_id, customer_name);

create table if not exists crm_points_ledgers (
  ledger_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  customer_id text,
  customer_name text not null,
  change_points bigint not null default 0,
  balance_points bigint not null default 0,
  reason text not null,
  ref_type text,
  ref_id text,
  created_at timestamptz not null default now()
);

create index if not exists idx_crm_points_ledgers_tenant_time
  on crm_points_ledgers(tenant_id, created_at desc);

create table if not exists crm_stored_value_ledgers (
  ledger_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  customer_id text,
  customer_name text not null,
  change_cent bigint not null default 0,
  balance_cent bigint not null default 0,
  reason text not null,
  ref_type text,
  ref_id text,
  created_at timestamptz not null default now()
);

create index if not exists idx_crm_stored_value_ledgers_tenant_time
  on crm_stored_value_ledgers(tenant_id, created_at desc);

