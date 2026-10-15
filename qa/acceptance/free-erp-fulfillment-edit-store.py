"""Apply scoped replacements only when each original function is still unchanged."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
path = ROOT / 'crates/erp-repo/src/fulfillment/store.rs'
original = (ROOT / 'qa/acceptance/free-erp-fulfillment-before-scoped-replacement.rs.txt').read_text(encoding='utf-8-sig')
current = path.read_text(encoding='utf-8-sig')


def replace(start, end, text):
    global current
    # Match free functions, not the indented public forwarding methods.
    start='\n'+start
    end='\n'+end
    before = original[original.index(start):original.index(end, original.index(start))]
    actual = current[current.index(start):current.index(end, current.index(start))]
    if actual != before:
        raise SystemExit(f'function changed concurrently; refusing overwrite: {start}')
    current = current.replace(actual, '\n'+text + '\n', 1)


replace('async fn build_reservation_plan(', 'async fn fetch_warehouse_rules(', r'''async fn build_reservation_plan(
    pool: &PgPool,
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    order_id: OrderId,
    items: &[OrderItemRow],
    resend_key: Option<uuid::Uuid>,
) -> anyhow::Result<Vec<ReservationGroup>> {
    if items.is_empty() || items.len()>1000 {
        return Err(InventoryError::ReservationComponentsLimit.into());
    }
    let mut parsed=Vec::new();
    let mut seen=HashSet::new();
    for item in items {
        let sku=uuid::Uuid::parse_str(item.sku_id.as_deref().ok_or(InventoryError::ReservationMismatch)?)
            .map_err(|_|InventoryError::ReservationMismatch)?;
        if sku.is_nil() || item.qty<=0 {return Err(InventoryError::ReservationMismatch.into());}
        erp_domain::quantity::ScaledQty::from_legacy_whole_units(item.qty)?;
        let base=order_item_uuid(order_id,&item.line_id);
        let root=match resend_key {Some(key)=>resend_order_item_id(base,key),None=>base};
        if !seen.insert(root) {return Err(InventoryError::ReservationMismatch.into());}
        parsed.push((sku,item.qty,base,root));
    }
    let roots:Vec<_>=parsed.iter().map(|(_,_,_,root)|*root).collect();
    lock_order_roots(tx,tenant_id,&roots).await?;
    let repo=InventoryRepo::new(pool);
    let mut groups=Vec::new();
    for (sku,qty,base,root) in parsed {
        let kind:Option<String>=sqlx::query_scalar("select sku_kind from skus where tenant_id=$1 and sku_id=$2")
            .bind(tenant_id.0).bind(sku).fetch_optional(&mut **tx).await?;
        let kind=kind.ok_or(InventoryError::ReservationMismatch)?;
        let frozen=repo.frozen_order_expansion(tx,tenant_id,root,sku,qty,Some(order_id.0)).await?;
        let (saved_warehouse_id,mut expansion)=if let Some((warehouse,expansion))=frozen {
            (Some(warehouse),expansion)
        }else{
            // Resend namespaces are new stock intents, but their component
            // identity comes from the original applied/order definition.
            let original=if resend_key.is_some(){
                repo.frozen_order_expansion(tx,tenant_id,base,sku,qty,Some(order_id.0)).await?
                    .map(|(_,expansion)|expansion)
            }else{None};
            let expansion=if let Some(original)=original{original}else{
                repo.expand_order_item(tx,tenant_id,sku,base,qty,Some(order_id.0)).await?
            };
            (None,expansion)
        };
        if expansion.items.is_empty() || expansion.items.len()>1000 {
            return Err(InventoryError::ReservationComponentsLimit.into());
        }
        if resend_key.is_some() && saved_warehouse_id.is_none() {
            for item in &mut expansion.items {
                item.order_item_id=if expansion.definition_kind=="DIRECT"{root}
                    else{component_order_item_id(root,item.sku_id)};
            }
        }
        let special_split=is_special_sku(&kind);
        let mut physical=Vec::new();
        for item in expansion.items {
            let weight:Option<i32>=sqlx::query_scalar("select weight_gram from skus where tenant_id=$1 and sku_id=$2")
                .bind(tenant_id.0).bind(item.sku_id).fetch_optional(&mut **tx).await?.flatten();
            physical.push(ReservationItem{order_item_id:item.order_item_id,sku_id:item.sku_id,qty:item.qty,
                special_split,weight_gram:weight,bom_version:item.bom_version});
        }
        physical.sort_by_key(|item|(item.sku_id,item.order_item_id));
        groups.push(ReservationGroup{order_id:order_id.0,root_order_item_id:root,parent_sku_id:sku,
            original_qty:qty,definition_kind:expansion.definition_kind,saved_warehouse_id,items:physical});
    }
    Ok(groups)
}''')

replace('async fn reserve_item_group(', 'async fn create_shipments(', r'''async fn reserve_item_group(
    pool:&PgPool,
    tx:&mut Transaction<'_,Postgres>,
    tenant_id:TenantId,
    warehouse_id:uuid::Uuid,
    group:&ReservationGroup,
) -> Result<Vec<AllocationItem>,ReservationFailure> {
    let service=crate::inventory::InventoryService::new(pool);
    // All candidate physical keys are already guarded in global order. Check
    // exact available whole units; a 0.9 residual is neither zero nor one.
    for item in &group.items {
        let existing:bool=sqlx::query_scalar("select exists(select 1 from inventory_reservations where tenant_id=$1 and order_item_id=$2)")
            .bind(tenant_id.0).bind(item.order_item_id).fetch_one(&mut **tx).await
            .map_err(|e|ReservationFailure::Error(e.into()))?;
        if !existing {
            let balance=service.get_precise_balance_with_tx(tx,crate::inventory_precision::InventoryQuantityKey{
                tenant_id,warehouse_id,sku_id:item.sku_id}).await.map_err(ReservationFailure::Error)?;
            let available=match balance{
                Some(balance)=>balance.whole_projection().map_err(ReservationFailure::Error)?.available(),None=>0,
            };
            if available<item.qty {
                return Err(ReservationFailure::Insufficient{shortage:AllocationShortage{
                    order_item_id:item.order_item_id,sku_id:item.sku_id,requested:item.qty,available}});
            }
        }
    }
    let prepared=ReservationExpansion{definition_kind:group.definition_kind,items:group.items.iter().map(|item|
        crate::inventory::ReservationItem{sku_id:item.sku_id,order_item_id:item.order_item_id,
            qty:item.qty,bom_version:item.bom_version}).collect()};
    InventoryRepo::new(pool).reserve_frozen_order_item_with_tx(tx,InventoryReservationInput{
        tenant_id,warehouse_id,sku_id:group.parent_sku_id,order_item_id:group.root_order_item_id,
        order_id:Some(group.order_id),qty:group.original_qty},Some(prepared))
        .await.map_err(ReservationFailure::Error)?;
    Ok(group.items.iter().map(|item|AllocationItem{order_item_id:item.order_item_id,sku_id:item.sku_id,
        qty:item.qty,warehouse_id,split_key:if item.special_split{format!("SPECIAL-{}",item.sku_id)}else{"NORMAL".into()},
        weight_gram:item.weight_gram}).collect())
}''')

replace('async fn consume_reservations(', 'async fn release_order_reservations(', r'''async fn consume_reservations(
    pool:&PgPool,
    tx:&mut Transaction<'_,Postgres>,
    tenant_id:TenantId,
    order_id:uuid::Uuid,
    warehouse_id:uuid::Uuid,
    items:Vec<OmsShipmentItem>,
) -> anyhow::Result<()> {
    if items.is_empty(){return Err(InventoryError::ReservationMismatch.into());}
    let applied:Vec<_>=items.into_iter().map(|item|AppliedOrderItem{
        warehouse_id,sku_id:item.sku_id,order_item_id:item.order_item_id,order_id,qty:item.qty}).collect();
    apply_order_terminal_items(pool,tx,tenant_id,&applied,true).await?;
    Ok(())
}''')

replace('async fn release_order_reservations(', 'async fn request_shipment_cancel(', r'''async fn release_order_reservations(
    pool:&PgPool,
    tenant_id:TenantId,
    order_id:OrderId,
    mark_order_cancelled:bool,
) -> anyhow::Result<ReleaseResult> {
    let mut tx:Transaction<'_,Postgres>=pool.begin().await?;
    lock_fulfillment_order(&mut tx,tenant_id,order_id.0).await?;
    #[derive(sqlx::FromRow)]
    struct Row {order_item_id:uuid::Uuid,warehouse_id:uuid::Uuid,sku_id:uuid::Uuid,qty:i64,status:String}
    let rows:Vec<Row>=sqlx::query_as("select order_item_id,warehouse_id,sku_id,qty,status
        from inventory_reservations where tenant_id=$1 and order_id=$2
        order by warehouse_id,sku_id,order_item_id limit 4097")
        .bind(tenant_id.0).bind(order_id.0).fetch_all(&mut *tx).await?;
    if rows.len()>MAX_APPLIED_ORDER_ITEMS {return Err(InventoryError::ReservationComponentsLimit.into());}
    let consumed=rows.iter().any(|row|row.status=="CONSUMED");
    let items:Vec<_>=rows.into_iter().map(|row|AppliedOrderItem{warehouse_id:row.warehouse_id,
        sku_id:row.sku_id,order_item_id:row.order_item_id,order_id:order_id.0,qty:row.qty}).collect();
    let released=apply_order_terminal_items(pool,&mut tx,tenant_id,&items,false).await?;
    if mark_order_cancelled && released>0 && !consumed {
        mark_order_released(&mut tx,tenant_id,order_id).await?;
    }
    tx.commit().await?;
    Ok(ReleaseResult{released})
}

async fn lock_fulfillment_order(
    tx:&mut Transaction<'_,Postgres>,tenant:TenantId,order:uuid::Uuid,
) -> anyhow::Result<()> {
    if tenant.0.is_nil() || order.is_nil(){return Err(InventoryError::ReservationMismatch.into());}
    sqlx::query("select set_config('statement_timeout','10s',true),set_config('lock_timeout','5s',true)")
        .execute(&mut **tx).await?;
    sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
        .bind(tenant.to_string()).bind(format!("fulfillment-order:{order}"))
        .execute(&mut **tx).await?;
    Ok(())
}''')

path.write_text(current, encoding='utf8')
print('Four original OMS stock functions replaced with scoped source/transaction kernels.')
