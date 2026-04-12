-- 0079: AI-1 platform integration fields

do $$
begin
  if exists (
    select 1
    from information_schema.columns
    where table_schema = 'public'
      and table_name = 'platform_tokens'
      and column_name = 'last_refresh_at'
  ) then
    alter table platform_tokens rename column last_refresh_at to last_refreshed_at;
  end if;
end $$;

alter table platform_tokens
  add column if not exists token_status text not null default 'ACTIVE',
  add column if not exists refresh_fail_count int not null default 0,
  add column if not exists last_refreshed_at timestamptz;

update platform_tokens
set token_status = case
  when expires_at is not null and expires_at > now() then 'ACTIVE'
  else 'EXPIRED'
end
where token_status is null or token_status = '';

update platform_tokens
set refresh_fail_count = 0
where refresh_fail_count is null;

alter table inbox_events
  add column if not exists verify_status text not null default 'PASSED',
  add column if not exists failed_reason text not null default '';

alter table channel_sku_mappings
  add column if not exists conflict_reason text not null default '';

do $$
begin
  if exists (
    select 1
    from information_schema.columns
    where table_schema = 'public'
      and table_name = 'sync_jobs'
      and column_name = 'last_time'
  ) then
    alter table sync_jobs rename column last_time to watermark;
  end if;
end $$;

alter table sync_jobs
  add column if not exists watermark timestamptz,
  add column if not exists last_success_at timestamptz;

update sync_jobs
set last_success_at = updated_at
where last_success_at is null and status = 'SUCCESS';

alter table stock_sync_jobs
  add column if not exists last_success_at timestamptz;

update stock_sync_jobs
set last_success_at = updated_at
where last_success_at is null and status = 'SENT';
