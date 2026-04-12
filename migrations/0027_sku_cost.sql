-- 0027: SKU cost price

alter table skus
  add column if not exists cost_cent bigint;
