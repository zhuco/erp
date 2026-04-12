-- 0077: Product allow negative stock flag

alter table products
  add column if not exists allow_negative_stock boolean not null default false;
