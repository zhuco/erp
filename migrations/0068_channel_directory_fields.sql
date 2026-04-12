alter table channels
  add column if not exists name text not null default '',
  add column if not exists department text,
  add column if not exists owner text,
  add column if not exists allow_manual_order boolean not null default false,
  add column if not exists allow_manual_after_sales boolean not null default false,
  add column if not exists default_warehouse_id uuid references warehouses(warehouse_id),
  add column if not exists commission_formula text,
  add column if not exists status text not null default 'ACTIVE';

alter table channels
  drop constraint if exists channels_tenant_id_platform_key;

update channels
set name = platform
where name = '';

create index if not exists idx_channels_tenant_status
  on channels(tenant_id, status);
