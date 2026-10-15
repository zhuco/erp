-- Protected QA prerequisites, copied from 0001_init, 0015_feature_flags,
-- 0030_system_management and 0038_reports_exports. No fixture reset/backfill.
\set ON_ERROR_STOP on
begin;
do $$ begin
  if current_database() <> 'erp_shared_contract_test' or current_user <> 'erp_grid_test'
     or inet_server_addr() <> '127.0.0.1'::inet or inet_server_port() <> 15438 then
    raise exception 'inventory export QA admission rejected';
  end if;
end $$;
create table if not exists roles (
  role_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id), name text not null,
  created_at timestamptz not null default now(), unique(tenant_id,name)
);
create table if not exists user_roles (
  tenant_id uuid not null references tenants(tenant_id), user_id uuid not null references users(user_id),
  role_id uuid not null references roles(role_id), created_at timestamptz not null default now(),
  primary key(tenant_id,user_id,role_id)
);
create table if not exists role_permissions (
  tenant_id uuid not null references tenants(tenant_id), role_id uuid not null references roles(role_id),
  perm_code text not null, created_at timestamptz not null default now(),
  primary key(tenant_id,role_id,perm_code)
);
create index if not exists idx_role_permissions_perm on role_permissions(tenant_id,perm_code);
create table if not exists feature_flags (
  tenant_id uuid not null, scope text not null default 'GLOBAL', shop_id uuid, key text not null,
  enabled boolean not null default false, created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);
create unique index if not exists idx_feature_flags_unique on feature_flags(
  tenant_id,scope,coalesce(shop_id,'00000000-0000-0000-0000-000000000000'::uuid),key
);
create index if not exists idx_feature_flags_lookup on feature_flags(tenant_id,scope,shop_id,key) where enabled=true;
create table if not exists audit_logs (
  audit_id uuid primary key default gen_random_uuid(), tenant_id uuid not null references tenants(tenant_id),
  user_id uuid, action text not null, entity_type text not null, entity_id text,
  before_json jsonb, after_json jsonb, ip text, ua text, created_at timestamptz not null default now()
);
create index if not exists idx_audit_logs_tenant_created on audit_logs(tenant_id,created_at desc,audit_id desc);
create index if not exists idx_audit_logs_tenant_action on audit_logs(tenant_id,action);
create index if not exists idx_audit_logs_tenant_entity on audit_logs(tenant_id,entity_type,entity_id);
create index if not exists idx_audit_logs_tenant_user on audit_logs(tenant_id,user_id,created_at desc);
create table if not exists export_jobs (
  job_id uuid primary key default gen_random_uuid(), tenant_id uuid not null references tenants(tenant_id),
  job_type text not null, params_json jsonb not null default '{}'::jsonb,
  status text not null default 'PENDING', output_url text, progress int not null default 0,
  attempts int not null default 0, last_error text not null default '',
  created_at timestamptz not null default now(), updated_at timestamptz not null default now()
);
create index if not exists idx_export_jobs_tenant_status on export_jobs(tenant_id,status,created_at desc);
create index if not exists idx_export_jobs_tenant_type on export_jobs(tenant_id,job_type);
commit;
