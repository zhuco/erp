alter table if exists shops
  add column if not exists default_carrier_code text;

alter table if exists print_devices
  add column if not exists is_default boolean not null default false;

alter table if exists print_templates
  add column if not exists is_default boolean not null default false;
