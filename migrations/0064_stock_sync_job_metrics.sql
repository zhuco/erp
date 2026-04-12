-- 0053: Stock sync job metrics (duration + counts)

alter table stock_sync_jobs
  add column if not exists success_count int not null default 0,
  add column if not exists failure_count int not null default 0,
  add column if not exists last_duration_ms int not null default 0;
