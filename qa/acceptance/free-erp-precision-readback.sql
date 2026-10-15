-- Independent read-only evidence. Identity must match the registered QA database.
\set ON_ERROR_STOP on
begin transaction read only;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'Protected QA identity required';
 end if;
end $$;
select json_build_object(
 'database',current_database(),'role',current_user,
 'host',inet_server_addr(),'port',inet_server_port(),
 'tenants',coalesce((
  select json_agg(json_build_object(
   'tenant_id',t.tenant_id,
   'balances',coalesce((
    select json_agg(json_build_object(
     'warehouse_id',b.warehouse_id,'sku_id',b.sku_id,
     'legacy_on_hand',b.on_hand::text,'legacy_reserved',b.reserved::text,
     'legacy_frozen',b.frozen::text,'legacy_in_transit',b.in_transit::text,
     'on_hand_micros',b.on_hand_micros::text,'reserved_micros',b.reserved_micros::text,
     'frozen_micros',b.frozen_micros::text,'in_transit_micros',b.in_transit_micros::text,
     'available_micros',(b.on_hand_micros::numeric-b.reserved_micros-b.frozen_micros)::text,
     'ledger',(
      select json_build_object(
       'count',count(*),
       'on_hand_micros',coalesce(sum(coalesce(i.delta_on_hand_micros::numeric,i.delta_on_hand::numeric*1000000)),0)::text,
       'reserved_micros',coalesce(sum(coalesce(i.delta_reserved_micros::numeric,i.delta_reserved::numeric*1000000)),0)::text,
       'frozen_micros',coalesce(sum(coalesce(i.delta_frozen_micros::numeric,i.delta_frozen::numeric*1000000)),0)::text,
       'in_transit_micros',coalesce(sum(coalesce(i.delta_in_transit_micros::numeric,i.delta_in_transit::numeric*1000000)),0)::text)
      from inventory_ledgers i where i.tenant_id=b.tenant_id
       and i.warehouse_id=b.warehouse_id and i.sku_id=b.sku_id),
     'source_map_count',(select count(*) from inventory_quantity_source_maps s
       join inventory_ledgers i on i.ledger_id=s.inventory_ledger_id
       where s.tenant_id=b.tenant_id and i.warehouse_id=b.warehouse_id and i.sku_id=b.sku_id)
    ) order by b.warehouse_id,b.sku_id)
    from inventory_balances b where b.tenant_id=t.tenant_id),'[]'::json),
   'reservations',coalesce((
    select json_agg(json_build_object(
     'reservation_id',r.reservation_id,'sku_id',r.sku_id,
     'source_type',r.source_type,'source_id',r.source_id,'status',r.status,
     'qty_micros',r.qty_micros::text,'original_qty_micros',r.original_qty_micros::text)
     order by r.reservation_id)
    from inventory_reservations r where r.tenant_id=t.tenant_id),'[]'::json),
   'batch_balances',coalesce((
    select json_agg(json_build_object('batch_id',b.batch_id,
     'on_hand_micros',b.on_hand_micros::text,'reserved_micros',b.reserved_micros::text,
     'batch_no',s.batch_no,'expire_at',s.expire_at))
    from inventory_batch_balances b left join sku_batches s
     on s.tenant_id=b.tenant_id and s.sku_id=b.sku_id and s.batch_id=b.batch_id
    where b.tenant_id=t.tenant_id),'[]'::json),
   'sku_count',(select count(*) from skus where tenant_id=t.tenant_id),
   'orphan_ledger_count',(select count(*) from inventory_ledgers i where i.tenant_id=t.tenant_id
    and not exists(select 1 from inventory_balances b where b.tenant_id=i.tenant_id
     and b.warehouse_id=i.warehouse_id and b.sku_id=i.sku_id)),
   'manufacturing_sources',coalesce((
    select json_agg(json_build_object(
     'ledger_id',m.ledger_id,'move_id',m.move_id,'reason',m.reason_type,
     'delta_micros',m.delta_micros::text,
     'inventory_source_count',(select count(*) from inventory_ledgers i
      where i.tenant_id=m.tenant_id and i.warehouse_id=m.warehouse_id and i.sku_id=m.sku_id
       and i.ref_type='mfg_material_move' and i.ref_id=m.move_id::text
       and coalesce(i.delta_on_hand_micros::numeric,i.delta_on_hand::numeric*1000000)=m.delta_micros))
     order by m.ledger_id)
   from mfg_qty_ledgers m where m.tenant_id=t.tenant_id and m.delta_micros<>0),'[]'::json),
   'shared_sources',coalesce((
    select json_agg(json_build_object(
     'ledger_id',s.ledger_id,'bucket',s.bucket,'delta_micros',s.delta_micros::text,
     'inventory_source_count',(select count(*) from inventory_ledgers i
      where i.tenant_id=s.tenant_id and i.warehouse_id=s.warehouse_id and i.sku_id=s.sku_id
       and i.ref_type='shared_stock_ledger' and i.ref_id=s.ledger_id::text
       and case s.bucket
        when 'ON_HAND' then coalesce(i.delta_on_hand_micros::numeric,i.delta_on_hand::numeric*1000000)
        when 'RESERVED' then coalesce(i.delta_reserved_micros::numeric,i.delta_reserved::numeric*1000000)
        when 'FROZEN' then coalesce(i.delta_frozen_micros::numeric,i.delta_frozen::numeric*1000000)
        when 'IN_TRANSIT' then coalesce(i.delta_in_transit_micros::numeric,i.delta_in_transit::numeric*1000000)
       end=s.delta_micros)) order by s.ledger_id)
    from shared_stock_ledgers s where s.tenant_id=t.tenant_id and s.delta_micros<>0),'[]'::json),
   'commands',coalesce((
    select json_agg(json_build_object('request_id',c.request_id,'status',c.status,'effect_id',c.effect_id)
     order by c.request_id)
    from command_receipts c where c.tenant_id=t.tenant_id),'[]'::json),
   'activation_count',(select count(*) from inventory_quantity_activations a where a.tenant_id=t.tenant_id),
   'shared_stock_count',(select count(*) from shared_stock_ledgers s where s.tenant_id=t.tenant_id)
  ) order by t.tenant_id)
  from tenants t where t.name='QA-CANONICAL-PRECISION'
 ),'[]'::json)
);
rollback;
