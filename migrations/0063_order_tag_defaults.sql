-- 0053: Order tag default/auto-apply rules

alter table order_tag_definitions
  add column if not exists auto_apply boolean not null default false,
  add column if not exists conditions_json jsonb not null default '[]'::jsonb;

create index if not exists idx_order_tag_defs_auto
  on order_tag_definitions(tenant_id, auto_apply, status, priority desc);
