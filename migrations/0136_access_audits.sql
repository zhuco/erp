-- Authorization binds tenant, actor, permission, and warehouse scope.
-- Audit stores a short summary. A frozen task is checked again at download.

create table if not exists access_grants (
  tenant_id uuid not null,
  actor_user_id text not null,
  permission text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, actor_user_id, permission)
);

create table if not exists access_warehouses (
  tenant_id uuid not null,
  actor_user_id text not null,
  warehouse_id uuid not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, actor_user_id, warehouse_id)
);

create table if not exists access_objects (
  tenant_id uuid not null,
  object_id uuid not null,
  object_kind text not null,
  warehouse_id uuid not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, object_id)
);

create table if not exists access_audits (
  tenant_id uuid not null,
  audit_id uuid not null,
  actor_user_id text not null,
  object_type text not null,
  object_id text not null,
  action text not null,
  summary text not null,
  request_id text not null,
  result text not null,
  retention_days integer not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, audit_id)
);

create table if not exists frozen_task_scopes (
  tenant_id uuid not null,
  task_id uuid not null,
  actor_user_id text not null,
  permission text not null,
  warehouse_id uuid not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, task_id)
);
