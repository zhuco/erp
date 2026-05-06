use super::*;

pub(super) async fn bind_serials_for_shipment(
    state: &AppState,
    tenant_id: TenantId,
    shipment_id: uuid::Uuid,
    order_id: OrderId,
    serials: Vec<ShipmentSerialEntry>,
) -> Result<Vec<SkuSerial>, axum::response::Response> {
    let mut bound = Vec::new();
    for entry in serials {
        if entry.serial_nos.is_empty() {
            return Err(bad_request("serial_nos required").into_response());
        }
        let sku_id = match entry.sku_id.as_deref() {
            Some(raw) => match uuid::Uuid::parse_str(raw) {
                Ok(v) => v,
                Err(_) => return Err(bad_request("bad sku_id").into_response()),
            },
            None => return Err(bad_request("sku_id required").into_response()),
        };

        let base_order_item_id = order_item_uuid(uuid::Uuid::from(order_id), &entry.line_id);
        let order_item_id = match resolve_serial_order_item_id(
            state.db.pool(),
            tenant_id,
            order_id,
            base_order_item_id,
            sku_id,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response())
            }
        };

        let serials = match state
            .sku_serials_repo
            .bind_serials_to_shipment(RepoShipmentSerialBindInput {
                tenant_id,
                shipment_id,
                order_id: uuid::Uuid::from(order_id),
                order_item_id,
                sku_id,
                serial_nos: entry.serial_nos,
            })
            .await
        {
            Ok(v) => v,
            Err(e) => return Err(serial_bind_error_response(e)),
        };
        bound.extend(serials);
    }
    Ok(bound)
}

pub(super) async fn resolve_serial_order_item_id(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    order_id: OrderId,
    base_order_item_id: uuid::Uuid,
    sku_id: uuid::Uuid,
) -> anyhow::Result<uuid::Uuid> {
    let exists: Option<i32> = sqlx::query_scalar(
        r#"
        select 1
        from order_item_components
        where tenant_id = $1
          and order_id = $2
          and order_item_id = $3
          and component_sku_id = $4
        limit 1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .bind(base_order_item_id)
    .bind(sku_id)
    .fetch_optional(pool)
    .await?;

    if exists.is_some() {
        Ok(component_order_item_id(base_order_item_id, sku_id))
    } else {
        Ok(base_order_item_id)
    }
}

pub(super) fn order_item_uuid(order_id: uuid::Uuid, line_id: &str) -> uuid::Uuid {
    uuid::Uuid::parse_str(line_id)
        .unwrap_or_else(|_| uuid::Uuid::new_v5(&order_id, line_id.as_bytes()))
}

pub(super) fn component_order_item_id(
    order_item_id: uuid::Uuid,
    component_sku_id: uuid::Uuid,
) -> uuid::Uuid {
    uuid::Uuid::new_v5(&order_item_id, component_sku_id.as_bytes())
}
