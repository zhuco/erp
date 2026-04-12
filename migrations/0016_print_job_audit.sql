-- 0013: print job audit fields alignment

alter table if exists print_jobs
  add column if not exists template_id uuid,
  add column if not exists template_version int not null default 1,
  add column if not exists requested_by_user_id uuid,
  add column if not exists device_fingerprint text;

do $$
begin
  if exists (
    select 1
    from information_schema.columns
    where table_name = 'print_jobs' and column_name = 'ref_id'
  ) then
    update print_jobs
    set template_id = ref_id
    where template_id is null;
  end if;
end $$;

update print_jobs
set template_id = gen_random_uuid()
where template_id is null;

update print_jobs
set requested_by_user_id = '00000000-0000-0000-0000-000000000000'::uuid
where requested_by_user_id is null;

alter table if exists print_jobs
  alter column template_id set not null,
  alter column requested_by_user_id
    set default '00000000-0000-0000-0000-000000000000'::uuid,
  alter column requested_by_user_id set not null;

do $$
begin
  if exists (
    select 1
    from information_schema.columns
    where table_name = 'print_jobs' and column_name = 'shop_id'
  ) then
    alter table print_jobs alter column shop_id drop not null;
  end if;
  if exists (
    select 1
    from information_schema.columns
    where table_name = 'print_jobs' and column_name = 'type'
  ) then
    alter table print_jobs alter column type drop not null;
  end if;
  if exists (
    select 1
    from information_schema.columns
    where table_name = 'print_jobs' and column_name = 'ref_type'
  ) then
    alter table print_jobs alter column ref_type drop not null;
  end if;
  if exists (
    select 1
    from information_schema.columns
    where table_name = 'print_jobs' and column_name = 'ref_id'
  ) then
    alter table print_jobs alter column ref_id drop not null;
  end if;
end $$;
