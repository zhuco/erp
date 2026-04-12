-- 0025: Event chain reliability (outbox attempts + shipment status ordering)

do $$
begin
  if exists (
    select 1
    from information_schema.columns
    where table_name = 'outbox_events'
      and column_name = 'try_count'
  ) and not exists (
    select 1
    from information_schema.columns
    where table_name = 'outbox_events'
      and column_name = 'attempts'
  ) then
    alter table outbox_events rename column try_count to attempts;
  end if;
end $$;

alter table shipments
  add column if not exists status_version int not null default 0;

update shipments
set status_version = case status
  when 'REQUESTED' then 10
  when 'FAILED' then 20
  when 'SUCCESS' then 30
  else 0
end
where status_version = 0;
