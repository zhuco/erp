-- 0074: Logistics fulfillment extensions

create table if not exists logistics_interfaces (
  interface_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  carrier_code text not null,
  carrier_name text not null,
  api_base_url text not null,
  template_name text not null,
  status text not null default 'HEALTHY',
  last_checked_at timestamptz,
  retry_policy text not null,
  alert_channel text not null,
  approval_status text not null default 'PENDING',
  approved_by text,
  approved_at timestamptz,
  notes text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, carrier_code)
);

create index if not exists idx_logistics_interfaces_tenant_status
  on logistics_interfaces(tenant_id, status, updated_at desc);

create table if not exists logistics_interface_approvals (
  approval_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  interface_id uuid not null references logistics_interfaces(interface_id) on delete cascade,
  action text not null,
  operator text not null,
  comment text,
  created_at timestamptz not null default now()
);

create index if not exists idx_logistics_interface_approvals_tenant
  on logistics_interface_approvals(tenant_id, interface_id, created_at desc);

create table if not exists logistics_exception_types (
  tenant_id uuid not null references tenants(tenant_id),
  type_code text not null,
  name text not null,
  sla_hours int not null default 6,
  severity text not null default 'MEDIUM',
  enabled boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, type_code)
);

create table if not exists logistics_exception_work_orders (
  ticket_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  order_id text not null,
  tracking_no text not null,
  carrier_code text not null,
  exception_type text not null,
  status text not null default 'OPEN',
  assignee text,
  created_at timestamptz not null default now(),
  due_at timestamptz,
  escalation_level int not null default 0,
  resolution text,
  resolved_at timestamptz
);

create index if not exists idx_logistics_exception_work_orders_tenant_status
  on logistics_exception_work_orders(tenant_id, status, due_at);

create table if not exists logistics_warehouse_strategies (
  strategy_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  name text not null,
  priority text not null default 'SPEED',
  enabled boolean not null default true,
  updated_by text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists logistics_warehouse_strategy_history (
  record_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  strategy_id uuid not null references logistics_warehouse_strategies(strategy_id) on delete cascade,
  summary text not null,
  updated_by text,
  updated_at timestamptz not null default now()
);

create index if not exists idx_logistics_warehouse_strategy_history
  on logistics_warehouse_strategy_history(tenant_id, strategy_id, updated_at desc);

create table if not exists logistics_warehouse_suggestions (
  suggestion_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  order_id text not null,
  destination text not null,
  recommended_warehouse text not null,
  eta_hours int not null,
  cost_cent bigint not null default 0,
  savings_cent bigint not null default 0,
  strategy_id uuid references logistics_warehouse_strategies(strategy_id),
  applied boolean not null default false,
  applied_at timestamptz,
  created_at timestamptz not null default now()
);

create index if not exists idx_logistics_warehouse_suggestions_tenant
  on logistics_warehouse_suggestions(tenant_id, applied, created_at desc);

create table if not exists logistics_installation_tasks (
  task_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  order_id text not null,
  customer_name text not null,
  address text not null,
  service_type text not null,
  assigned_team text,
  status text not null default 'SCHEDULED',
  remedy_ticket_id text,
  exception_note text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists logistics_installation_appointments (
  appointment_id uuid primary key default gen_random_uuid(),
  task_id uuid not null references logistics_installation_tasks(task_id) on delete cascade,
  scheduled_at timestamptz not null,
  status text not null default 'BOOKED',
  created_at timestamptz not null default now()
);

create index if not exists idx_logistics_installation_appointments_task
  on logistics_installation_appointments(task_id, created_at desc);

create table if not exists logistics_settlement_rules (
  tenant_id uuid primary key references tenants(tenant_id),
  weight_rounding text not null default '按 0.5KG 进位',
  min_fee_cent bigint not null default 0,
  oversize_fee_cent bigint not null default 0,
  overtime_fee_cent bigint not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists logistics_settlement_batches (
  batch_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  carrier text not null,
  period_start date not null,
  period_end date not null,
  total_shipments int not null default 0,
  expected_cent bigint not null default 0,
  actual_cent bigint not null default 0,
  diff_cent bigint not null default 0,
  status text not null default 'DRAFT',
  deductions text[] not null default '{}'::text[],
  payment_plan_id text,
  created_at timestamptz not null default now()
);

create index if not exists idx_logistics_settlement_batches_tenant
  on logistics_settlement_batches(tenant_id, status, period_end desc);
