-- Frozen legacy root requests and applied components. No historical backfill.
create table inventory_order_reservation_sources (
 source_id uuid primary key default gen_random_uuid(),
 tenant_id uuid not null references tenants(tenant_id),
 root_order_item_id uuid not null check(root_order_item_id<>'00000000-0000-0000-0000-000000000000'),
 warehouse_id uuid not null references warehouses(warehouse_id),
 sku_id uuid not null references skus(sku_id),
 order_id uuid check(order_id is null or order_id<>'00000000-0000-0000-0000-000000000000'),
 original_qty bigint not null check(original_qty between 1 and 9223372036854),
 component_count integer not null check(component_count between 1 and 1000),
 definition_kind text not null check(definition_kind in ('DIRECT','ORDER_COMPONENTS','SKU_BOM')),
 created_at timestamptz not null default now(),
 unique(tenant_id,root_order_item_id),
 unique(tenant_id,source_id)
);
alter table inventory_reservations add column reservation_source_id uuid;
alter table inventory_reservations add constraint inventory_order_reservation_source_fk
 foreign key(tenant_id,reservation_source_id) references inventory_order_reservation_sources(tenant_id,source_id);
create index inventory_unmapped_order_reservations
 on inventory_reservations(tenant_id,order_item_id) where reservation_source_id is null and order_item_id is not null;
create index inventory_order_reservation_children
 on inventory_reservations(tenant_id,reservation_source_id) where reservation_source_id is not null;
create table inventory_order_reservation_components (
 tenant_id uuid not null,
 source_id uuid not null,
 order_item_id uuid not null check(order_item_id<>'00000000-0000-0000-0000-000000000000'),
 sku_id uuid not null references skus(sku_id),
 qty bigint not null check(qty between 1 and 9223372036854),
 bom_version integer,
 primary key(tenant_id,source_id,sku_id),
 unique(tenant_id,order_item_id),
 foreign key(tenant_id,source_id) references inventory_order_reservation_sources(tenant_id,source_id),
 foreign key(tenant_id,order_item_id) references inventory_reservations(tenant_id,order_item_id)
  deferrable initially deferred
);

create function guard_inventory_order_reservation_root() returns trigger language plpgsql as $$
begin
 if tg_op<>'INSERT' then raise exception 'original order reservation root is immutable'; end if;
 if not exists(select 1 from warehouses where warehouse_id=new.warehouse_id and tenant_id=new.tenant_id)
   or not exists(select 1 from skus where sku_id=new.sku_id and tenant_id=new.tenant_id) then
  raise exception 'order reservation root must own its physical key';
 end if;
 return new;
end $$;
create trigger inventory_order_reservation_root_immutable before insert or update or delete
 on inventory_order_reservation_sources for each row execute function guard_inventory_order_reservation_root();

create function guard_inventory_order_reservation_component() returns trigger language plpgsql as $$
begin
 if tg_op<>'INSERT' then raise exception 'original order reservation component is immutable'; end if;
 if not exists(select 1 from skus where sku_id=new.sku_id and tenant_id=new.tenant_id) then
  raise exception 'order reservation component must own its SKU';
 end if;
 return new;
end $$;
create trigger inventory_order_reservation_component_immutable before insert or update or delete
 on inventory_order_reservation_components for each row execute function guard_inventory_order_reservation_component();

create function guard_inventory_order_reservation_attachment() returns trigger language plpgsql as $$
begin
 if tg_op='UPDATE' and new.reservation_source_id is distinct from old.reservation_source_id then
  raise exception 'original order reservation attachment is immutable; historical roots need explicit review';
 end if;
 return new;
end $$;
create trigger inventory_order_reservation_attachment_immutable before update
 on inventory_reservations for each row execute function guard_inventory_order_reservation_attachment();

create function validate_inventory_order_reservation_source(p_tenant uuid,p_source uuid) returns void language plpgsql as $$
declare r inventory_order_reservation_sources%rowtype; n bigint;
begin
 select * into r from inventory_order_reservation_sources where tenant_id=p_tenant and source_id=p_source;
 if not found then raise exception 'original order reservation root is missing'; end if;
 select count(*) into n from inventory_order_reservation_components where tenant_id=p_tenant and source_id=p_source;
 if n<>r.component_count then raise exception 'original order reservation component list is incomplete'; end if;
 if exists(
  select 1 from inventory_order_reservation_components c
  left join inventory_reservations s on s.tenant_id=c.tenant_id and s.order_item_id=c.order_item_id
  where c.tenant_id=p_tenant and c.source_id=p_source and (
   s.reservation_id is null or s.reservation_source_id is distinct from p_source
   or s.warehouse_id<>r.warehouse_id or s.sku_id<>c.sku_id
   or s.order_id is distinct from r.order_id or s.qty<>c.qty
   or s.qty_micros is distinct from (c.qty::numeric*1000000)::bigint
   or s.original_qty_micros is distinct from (c.qty::numeric*1000000)::bigint
   or s.source_type is not null or s.source_id is not null or s.location_id is not null
   or s.batch_code is not null or s.quality_status is not null
   or s.status not in ('RESERVED','RELEASED','CONSUMED')
   or (r.definition_kind='DIRECT' and (c.order_item_id<>r.root_order_item_id or c.sku_id<>r.sku_id or c.qty<>r.original_qty))))
 then raise exception 'original order reservation child differs from frozen physical intent'; end if;
 if exists(select 1 from inventory_reservations s where s.tenant_id=p_tenant and s.reservation_source_id=p_source
    and not exists(select 1 from inventory_order_reservation_components c where c.tenant_id=s.tenant_id and c.source_id=p_source and c.order_item_id=s.order_item_id))
 then raise exception 'order reservation has an unlisted child'; end if;
end $$;
create function validate_inventory_order_reservation_root_trigger() returns trigger language plpgsql as $$
begin
 perform validate_inventory_order_reservation_source(new.tenant_id,new.source_id); return null;
end $$;
create constraint trigger inventory_order_reservation_root_complete after insert
 on inventory_order_reservation_sources deferrable initially deferred for each row
 execute function validate_inventory_order_reservation_root_trigger();
create constraint trigger inventory_order_reservation_components_complete after insert
 on inventory_order_reservation_components deferrable initially deferred for each row
 execute function validate_inventory_order_reservation_root_trigger();
create function validate_inventory_order_reservation_child_trigger() returns trigger language plpgsql as $$
begin
 if new.reservation_source_id is not null then
  perform validate_inventory_order_reservation_source(new.tenant_id,new.reservation_source_id);
 end if;
 return null;
end $$;
create constraint trigger inventory_order_reservation_children_complete after insert or update
 on inventory_reservations deferrable initially deferred for each row
 execute function validate_inventory_order_reservation_child_trigger();
