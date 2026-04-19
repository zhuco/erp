-- 0094: Universal workflow engine (instances, todo center, callbacks, audit-friendly traces)

create table if not exists workflow_instances (
  instance_id uuid primary key,
  tenant_id uuid not null,
  workflow_code text not null,
  biz_domain text not null,
  biz_id text not null,
  status text not null,
  submitter_id uuid not null,
  current_step_seq int not null,
  total_steps int not null,
  callback_target_approved text not null,
  callback_target_rejected text not null,
  callback_target_withdrawn text not null,
  callback_payload jsonb not null default '{}'::jsonb,
  version bigint not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_workflow_instances_tenant_biz
  on workflow_instances(tenant_id, biz_domain, biz_id, status, updated_at desc);

create table if not exists workflow_submit_dedup (
  tenant_id uuid not null,
  biz_domain text not null,
  biz_id text not null,
  workflow_code text not null,
  request_id text not null,
  instance_id uuid not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, biz_domain, biz_id, workflow_code, request_id)
);

create table if not exists workflow_steps (
  step_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null,
  instance_id uuid not null,
  step_seq int not null,
  node_code text not null,
  assignee_user_id uuid not null,
  status text not null default 'PENDING',
  acted_by uuid,
  acted_at timestamptz,
  action_note text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, instance_id, step_seq, assignee_user_id)
);

create index if not exists idx_workflow_steps_instance_seq
  on workflow_steps(tenant_id, instance_id, step_seq, status);

create table if not exists workflow_todos (
  todo_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null,
  instance_id uuid not null,
  step_seq int not null,
  assignee_user_id uuid not null,
  todo_type text not null,
  status text not null default 'OPEN',
  completed_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, instance_id, step_seq, assignee_user_id, todo_type)
);

create index if not exists idx_workflow_todos_assignee
  on workflow_todos(tenant_id, assignee_user_id, status, created_at desc);

create table if not exists workflow_notifications (
  notification_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null,
  instance_id uuid not null,
  user_id uuid not null,
  channel text not null,
  template_code text not null,
  payload jsonb not null default '{}'::jsonb,
  status text not null default 'PENDING',
  created_at timestamptz not null default now()
);

create index if not exists idx_workflow_notifications_user
  on workflow_notifications(tenant_id, user_id, status, created_at desc);

create table if not exists workflow_action_dedup (
  tenant_id uuid not null,
  instance_id uuid not null,
  action text not null,
  request_id text not null,
  action_log_id uuid not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, instance_id, action, request_id)
);

create table if not exists workflow_action_logs (
  action_log_id uuid primary key,
  tenant_id uuid not null,
  instance_id uuid not null,
  action text not null,
  request_id text not null,
  actor_user_id uuid,
  from_status text not null,
  to_status text not null,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  unique (tenant_id, instance_id, action, request_id)
);

create index if not exists idx_workflow_action_logs_instance
  on workflow_action_logs(tenant_id, instance_id, created_at desc);

create table if not exists workflow_callbacks (
  callback_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null,
  instance_id uuid not null,
  biz_domain text not null,
  biz_id text not null,
  contract_version text not null,
  target_status text not null,
  payload jsonb not null default '{}'::jsonb,
  status text not null default 'PENDING',
  retry_count int not null default 0,
  timeout_at timestamptz,
  last_error text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, instance_id, target_status)
);

create index if not exists idx_workflow_callbacks_status
  on workflow_callbacks(tenant_id, status, updated_at desc);

create table if not exists workflow_callback_receipts (
  tenant_id uuid not null,
  callback_id uuid not null,
  request_id text not null,
  action_log_id uuid not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, callback_id, request_id)
);
