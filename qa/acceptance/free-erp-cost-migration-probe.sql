-- Test 0281 against its old table shape in an isolated, rolled-back QA schema.
-- The public schema is excluded from search_path throughout the probe.
begin;
do $$
begin
  if current_database() <> 'erp_shared_contract_test'
     or current_user <> 'erp_grid_test'
     or inet_server_port() <> 15438 then
    raise exception 'refusing a database outside the protected QA identity';
  end if;
end $$;
create schema qa_cost_order_migration_probe;
set local search_path = qa_cost_order_migration_probe;

create table shared_item_costs (
  tenant_id uuid not null,
  entry_id uuid not null,
  item_code text not null,
  kind text not null,
  qty_micros bigint not null,
  amount_cent bigint not null,
  unit_cent bigint not null,
  estimated boolean not null default false,
  algorithm text not null,
  source_ref text not null,
  period_code text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, entry_id)
);
insert into shared_item_costs values
  ('00000000-0000-4000-8000-000000000001','00000000-0000-4000-8000-000000000003','QA','RETURN',1000000,1,0,false,'MOVING_AVERAGE_V1','issue','2026-10','2000-01-03'),
  ('00000000-0000-4000-8000-000000000001','00000000-0000-4000-8000-000000000002','QA','ISSUE',3000000,2,0,false,'MOVING_AVERAGE_V1','issue','2026-10','2000-01-02'),
  ('00000000-0000-4000-8000-000000000001','00000000-0000-4000-8000-000000000001','QA','RECEIPT',3000000,2,0,false,'MOVING_AVERAGE_V1','','2026-10','2000-01-01');
create temporary table qa_cost_before as select * from shared_item_costs;

\i migrations/0281_cost_entry_order.sql
do $$
begin
  if (select array_agg(entry_sequence order by created_at,entry_id) from shared_item_costs) <> array[1,2,3]::bigint[] then
    raise exception 'historical order was not preserved';
  end if;
  if exists (
    select 1 from qa_cost_before b full join shared_item_costs a using(tenant_id,entry_id)
    where row(b.item_code,b.kind,b.qty_micros,b.amount_cent,b.unit_cent,b.estimated,b.algorithm,b.source_ref,b.period_code,b.created_at)
      is distinct from row(a.item_code,a.kind,a.qty_micros,a.amount_cent,a.unit_cent,a.estimated,a.algorithm,a.source_ref,a.period_code,a.created_at)
  ) then
    raise exception 'migration changed historical business facts';
  end if;
end $$;

insert into shared_item_costs(tenant_id,entry_id,item_code,kind,qty_micros,amount_cent,unit_cent,algorithm,source_ref,created_at)
values('00000000-0000-4000-8000-000000000001','00000000-0000-4000-8000-000000000004','QA','ADJUST',0,1,0,'MOVING_AVERAGE_V1','','1999-01-01');
\i migrations/0281_cost_entry_order.sql
do $$
begin
  if (select entry_sequence from shared_item_costs where kind='ADJUST') <> 4 then
    raise exception 'a later append did not receive the later sequence';
  end if;
  if (select array_agg(entry_sequence order by entry_sequence) from shared_item_costs) <> array[1,2,3,4]::bigint[] then
    raise exception 'migration replay rewrote ordering';
  end if;
  if nextval('shared_item_cost_entry_sequence') <> 5 then
    raise exception 'sequence restart did not follow committed entries';
  end if;
  raise notice 'PASS: historical facts/order, new append default, migration replay and sequence continuation';
end $$;
rollback;
