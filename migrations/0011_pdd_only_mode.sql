alter table tenants
  add column if not exists pdd_only_mode boolean not null default false;
