-- 0060: Stock count diff reason

alter table stock_count_items
  add column if not exists diff_reason text;
