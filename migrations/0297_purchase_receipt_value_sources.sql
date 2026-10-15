-- Original records stay unchanged. Every new receipt action retains its cost sources.
create table shared_receipt_value_commands (
  tenant_id uuid not null,
  request_id text not null,
  quality_id uuid not null,
  action text not null check (action in ('RELEASE','RETURN','ISSUE')),
  qty_micros bigint not null check (qty_micros > 0),
  amount_cent bigint not null check (amount_cent >= 0),
  result bigint not null check (result >= 0),
  payload_hash text not null,
  stock_id uuid not null,
  held_id uuid,
  balance_id uuid,
  return_id uuid,
  created_at timestamptz not null default now(),
  primary key (tenant_id,request_id),
  unique (tenant_id,stock_id),
  unique (tenant_id,balance_id),
  unique (tenant_id,return_id),
  foreign key (tenant_id,quality_id) references shared_receipt_quality(tenant_id,quality_id),
  foreign key (tenant_id,request_id) references command_receipts(tenant_id,request_id) deferrable initially deferred,
  foreign key (tenant_id,stock_id) references shared_stock_ledgers(tenant_id,ledger_id),
  foreign key (tenant_id,held_id) references shared_stock_ledgers(tenant_id,ledger_id),
  foreign key (tenant_id,balance_id) references partner_balances(tenant_id,balance_id),
  foreign key (tenant_id,return_id) references shared_supplier_returns(tenant_id,return_id),
  check ((action='RELEASE' and held_id is not null and balance_id is not null and return_id is null)
    or (action='RETURN' and held_id is null and balance_id is not null and return_id is not null)
    or (action='ISSUE' and held_id is null and balance_id is null and return_id is null))
);
create index shared_receipt_value_quality on shared_receipt_value_commands(tenant_id,quality_id,request_id);

create table shared_receipt_cost_lots (
  tenant_id uuid not null,
  quality_id uuid not null,
  lot_id uuid not null,
  ordinal bigserial not null,
  kind text not null check (kind in ('LEGACY_BALANCE','RELEASE')),
  request_id text,
  qty_micros bigint not null check (qty_micros >= 0),
  cost_cent bigint not null check (cost_cent >= 0),
  legacy_facts jsonb,
  created_at timestamptz not null default now(),
  primary key (tenant_id,quality_id,lot_id),
  unique (tenant_id,request_id),
  foreign key (tenant_id,quality_id) references shared_receipt_quality(tenant_id,quality_id),
  foreign key (tenant_id,request_id) references shared_receipt_value_commands(tenant_id,request_id) deferrable initially deferred,
  check ((kind='LEGACY_BALANCE' and request_id is null and legacy_facts is not null)
    or (kind='RELEASE' and request_id is not null and legacy_facts is null and qty_micros>0)),
  check (qty_micros>0 or cost_cent=0)
);
create unique index shared_receipt_cost_legacy on shared_receipt_cost_lots(tenant_id,quality_id) where kind='LEGACY_BALANCE';
create index shared_receipt_cost_order on shared_receipt_cost_lots(tenant_id,quality_id,ordinal);

create table shared_receipt_cost_allocations (
  tenant_id uuid not null,
  quality_id uuid not null,
  lot_id uuid not null,
  request_id text not null,
  ordinal bigserial not null,
  action text not null check (action in ('RETURN','ISSUE')),
  qty_micros bigint not null check (qty_micros > 0),
  cost_cent bigint not null check (cost_cent >= 0),
  created_at timestamptz not null default now(),
  primary key (tenant_id,request_id,lot_id),
  foreign key (tenant_id,quality_id,lot_id) references shared_receipt_cost_lots(tenant_id,quality_id,lot_id),
  foreign key (tenant_id,request_id) references shared_receipt_value_commands(tenant_id,request_id) deferrable initially deferred
);
create index shared_receipt_cost_allocated on shared_receipt_cost_allocations(tenant_id,quality_id,lot_id) include(qty_micros,cost_cent);

-- This fingerprint describes only pre-journal source rows, not the mutable quality projection.
-- ID/order/amounts/keys/timestamps are retained; adding a new tracked command does not change it.
create function receipt_value_legacy_facts(t uuid,q uuid) returns jsonb language sql stable as $$
  select jsonb_build_object(
    'stock',(select md5(coalesce(string_agg(to_jsonb(x)::text,'|' order by x.ledger_id),'')) from shared_stock_ledgers x
      where x.tenant_id=t and x.source_line=q::text and not exists
        (select 1 from shared_receipt_value_commands c where c.tenant_id=t and (c.stock_id=x.ledger_id or c.held_id=x.ledger_id))),
    'postings',(select md5(coalesce(string_agg((to_jsonb(x)-'settled_cent'-'due_date')::text,'|' order by x.balance_id),'')) from partner_balances x
      join shared_receipt_quality z on z.tenant_id=t and z.quality_id=q and x.source_id=z.receipt_id::text
      where x.tenant_id=t and x.source_type in ('PURCHASE_RELEASE','PURCHASE_RETURN') and not exists
        (select 1 from shared_receipt_value_commands c where c.tenant_id=t and c.balance_id=x.balance_id)),
    'returns',(select md5(coalesce(string_agg(to_jsonb(x)::text,'|' order by x.return_id),'')) from shared_supplier_returns x
      join shared_receipt_quality z on z.tenant_id=t and z.quality_id=q and x.receipt_id=z.receipt_id
      where x.tenant_id=t and not exists
        (select 1 from shared_receipt_value_commands c where c.tenant_id=t and c.return_id=x.return_id)))
$$;

-- Source comparisons also protect read/replay after a legacy source was changed.
create function receipt_value_integrity(t uuid,q uuid) returns boolean language sql stable as $$
select
  not exists (
    select 1 from shared_receipt_value_commands c
    join shared_receipt_quality z on z.tenant_id=c.tenant_id and z.quality_id=c.quality_id
    left join command_receipts r on r.tenant_id=c.tenant_id and r.request_id=c.request_id
    left join shared_stock_ledgers s on s.tenant_id=c.tenant_id and s.ledger_id=c.stock_id
    left join shared_stock_ledgers h on h.tenant_id=c.tenant_id and h.ledger_id=c.held_id
    left join partner_balances p on p.tenant_id=c.tenant_id and p.balance_id=c.balance_id
    left join shared_supplier_returns x on x.tenant_id=c.tenant_id and x.return_id=c.return_id
    where c.tenant_id=t and c.quality_id=q and (
      r.status is distinct from 'SUCCEEDED' or r.payload_hash is distinct from c.payload_hash
      or r.effect_id is distinct from c.result::text
      or s.source_line is distinct from q::text or s.request_id is distinct from c.request_id
      or s.warehouse_id is distinct from z.warehouse_id or s.location_id is distinct from z.location_id
      or s.sku_id is distinct from z.sku_id or s.batch_code is distinct from z.batch_code
      or s.bucket is distinct from 'ON_HAND' or s.quality_status is distinct from 'GOOD'
      or s.delta_micros is distinct from (case when c.action='RELEASE' then c.qty_micros else -c.qty_micros end)
      or (c.action='RELEASE' and (
        h.source_line is distinct from q::text or h.request_id is distinct from c.request_id
        or h.warehouse_id is distinct from z.warehouse_id or h.location_id is distinct from z.location_id
        or h.sku_id is distinct from z.sku_id or h.batch_code is distinct from z.batch_code
        or h.bucket is distinct from 'HELD' or h.quality_status is distinct from 'HELD' or h.delta_micros is distinct from -c.qty_micros
        or not exists (select 1 from shared_receipt_cost_lots l where l.tenant_id=t and l.quality_id=q and l.request_id=c.request_id
          and l.kind='RELEASE' and l.qty_micros=c.qty_micros and l.cost_cent=c.amount_cent)))
      or (c.action in ('RELEASE','RETURN') and (
        p.kind is distinct from 'AP' or p.currency is distinct from 'CNY' or p.partner_code is distinct from z.supplier_code
        or p.source_id is distinct from z.receipt_id::text
        or p.source_type is distinct from (case when c.action='RELEASE' then 'PURCHASE_RELEASE' else 'PURCHASE_RETURN' end)
        or p.amount_cent is distinct from (case when c.action='RELEASE' then c.amount_cent else -c.amount_cent end)))
      or (c.action='RETURN' and (x.receipt_id is distinct from z.receipt_id or x.qty_micros is distinct from c.qty_micros or x.cost_cent is distinct from c.amount_cent))
      or (c.action in ('RETURN','ISSUE') and (
        (select coalesce(sum(a.qty_micros),0) from shared_receipt_cost_allocations a where a.tenant_id=t and a.request_id=c.request_id)<>c.qty_micros
        or (select coalesce(sum(a.cost_cent),0) from shared_receipt_cost_allocations a where a.tenant_id=t and a.request_id=c.request_id)<>c.amount_cent
        or exists (select 1 from shared_receipt_cost_allocations a where a.tenant_id=t and a.request_id=c.request_id and (a.quality_id<>q or a.action<>c.action))))
    ))
  and not exists (
    select 1 from shared_receipt_cost_lots l
    cross join lateral (select coalesce(sum(a.qty_micros),0) qty,coalesce(sum(a.cost_cent),0) cost
      from shared_receipt_cost_allocations a where a.tenant_id=t and a.quality_id=q and a.lot_id=l.lot_id) a
    where l.tenant_id=t and l.quality_id=q and (a.qty>l.qty_micros or a.cost>l.cost_cent
      or (a.qty=l.qty_micros and a.cost<>l.cost_cent)
      or (l.kind='LEGACY_BALANCE' and l.legacy_facts->'fingerprint' is distinct from receipt_value_legacy_facts(t,q))))
  and not exists (
    select 1 from (
      select l.qty_micros,l.cost_cent,
        sum(a.qty_micros) over(partition by a.lot_id order by a.ordinal) qty,
        sum(a.cost_cent) over(partition by a.lot_id order by a.ordinal) cost
      from shared_receipt_cost_allocations a
      join shared_receipt_cost_lots l on l.tenant_id=a.tenant_id and l.quality_id=a.quality_id and l.lot_id=a.lot_id
      where a.tenant_id=t and a.quality_id=q
    ) x where x.qty_micros=0 or x.cost<>floor((x.cost_cent::numeric*x.qty+x.qty_micros/2)/nullif(x.qty_micros,0))
  )
$$;

create function keep_receipt_value_original() returns trigger language plpgsql as $$
begin raise exception 'receipt cost sources and allocations are immutable'; end $$;
create trigger receipt_value_original before update or delete on shared_receipt_value_commands for each row execute function keep_receipt_value_original();
create trigger receipt_cost_lot_original before update or delete on shared_receipt_cost_lots for each row execute function keep_receipt_value_original();
create trigger receipt_cost_allocation_original before update or delete on shared_receipt_cost_allocations for each row execute function keep_receipt_value_original();

create function check_receipt_value_commit() returns trigger language plpgsql as $$
begin
  if not receipt_value_integrity(new.tenant_id,new.quality_id) then
    raise exception 'receipt cost command lacks matching original stock, AP, return or allocation';
  end if;
  return null;
end $$;
create constraint trigger receipt_value_commit after insert on shared_receipt_value_commands
  deferrable initially deferred for each row execute function check_receipt_value_commit();
