-- Existing rows retain their unknown historical identity. New writes must name
-- their physical material and an original ISSUE/SUPPLEMENT for return/consume.
alter table mfg_material_moves add column if not exists warehouse_id uuid;
alter table mfg_material_moves add column if not exists sku_id uuid;
alter table mfg_material_moves add column if not exists batch_id uuid;
alter table mfg_material_moves add column if not exists original_issue_id uuid;
alter table mfg_material_moves add column if not exists material_issued_micros bigint;
alter table mfg_material_moves add column if not exists material_returned_micros bigint;
alter table mfg_material_moves add column if not exists material_consumed_micros bigint;
alter table mfg_material_moves add column if not exists material_shop_micros bigint;

create index if not exists mfg_material_identity_history
  on mfg_material_moves(tenant_id,work_order_id,warehouse_id,sku_id,batch_id);
create index if not exists mfg_material_original_issue
  on mfg_material_moves(tenant_id,original_issue_id);

create or replace function validate_new_material_identity() returns trigger
language plpgsql as $$
declare
  original mfg_material_moves%rowtype;
  already_spent numeric;
begin
  if new.warehouse_id is null or new.sku_id is null or new.qty_micros <= 0 then
    raise exception 'material warehouse, SKU and positive quantity are required';
  end if;
  if not exists(select 1 from warehouses where tenant_id=new.tenant_id and warehouse_id=new.warehouse_id)
     or not exists(select 1 from skus where tenant_id=new.tenant_id and sku_id=new.sku_id)
     or not exists(select 1 from mfg_work_orders where tenant_id=new.tenant_id and work_order_id=new.work_order_id) then
    raise exception 'material order, warehouse or SKU is outside the enterprise';
  end if;
  if new.batch_id is not null and not exists(
    select 1 from sku_batches where tenant_id=new.tenant_id and sku_id=new.sku_id and batch_id=new.batch_id
  ) then
    raise exception 'material batch is outside the selected SKU';
  end if;
  if new.kind in ('RETURN','CONSUME') then
    select * into original from mfg_material_moves
      where tenant_id=new.tenant_id and move_id=new.original_issue_id for update;
    if not found or original.kind not in ('ISSUE','SUPPLEMENT')
       or original.work_order_id<>new.work_order_id
       or original.warehouse_id is distinct from new.warehouse_id
       or original.sku_id is distinct from new.sku_id
       or original.batch_id is distinct from new.batch_id then
      raise exception 'original material issue does not match the selected material';
    end if;
    select coalesce(sum(qty_micros),0) into already_spent from mfg_material_moves
      where tenant_id=new.tenant_id and original_issue_id=new.original_issue_id
        and kind in ('RETURN','CONSUME');
    if already_spent+new.qty_micros>original.qty_micros then
      raise exception 'material return or consume exceeds the original issue';
    end if;
  elsif new.kind in ('ISSUE','SUPPLEMENT') then
    if new.original_issue_id is not null then
      raise exception 'new material issue cannot consume an original issue';
    end if;
  else
    raise exception 'unsupported material movement';
  end if;
  if new.kind in ('ISSUE','SUPPLEMENT','RETURN') and not exists(
    select 1 from mfg_qty_ledgers where tenant_id=new.tenant_id and move_id=new.move_id
      and warehouse_id=new.warehouse_id and sku_id=new.sku_id
      and delta_micros=case when new.kind='RETURN' then new.qty_micros else -new.qty_micros end
  ) then
    raise exception 'material movement requires its matching quantity ledger';
  end if;
  return new;
end $$;

drop trigger if exists mfg_material_identity_insert on mfg_material_moves;
create trigger mfg_material_identity_insert before insert on mfg_material_moves
  for each row execute function validate_new_material_identity();
