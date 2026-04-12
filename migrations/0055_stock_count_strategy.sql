-- 0049: Stock count strategy/dimension metadata

alter table stock_counts
  add column if not exists strategy text not null default 'FULL',
  add column if not exists dimension text not null default 'SKU',
  add column if not exists filters_json jsonb not null default '{}'::jsonb;
