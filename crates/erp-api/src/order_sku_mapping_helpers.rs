use super::*;

pub(crate) async fn apply_order_item_sku_mappings(
    pool: &PgPool,
    outbox_repo: &OutboxRepo<'_>,
    order: &mut OrderProjection,
) -> anyhow::Result<()> {
    for item in &mut order.items {
        let Some(platform_sku_id) = item.sku_id.clone() else {
            continue;
        };

        let mapped_sku_id = resolve_order_item_sku_mapping(
            pool,
            order.tenant_id,
            order.shop_id,
            order.platform,
            &platform_sku_id,
        )
        .await?;

        match mapped_sku_id {
            Some(mapped) => item.sku_id = Some(mapped),
            None => {
                item.sku_id = None;
                let payload = serde_json::json!({
                    "tenant_id": order.tenant_id,
                    "shop_id": order.shop_id,
                    "platform": order.platform,
                    "platform_sku_id": platform_sku_id,
                    "platform_order_id": order.platform_order_id,
                });
                outbox_repo
                    .enqueue(order.tenant_id, "MAPPING_MISSING", &payload)
                    .await?;
                warn!(
                    tenant_id = %order.tenant_id,
                    shop_id = %order.shop_id,
                    platform = %order.platform,
                    platform_sku_id = %platform_sku_id,
                    platform_order_id = %order.platform_order_id,
                    "sku mapping missing"
                );
            }
        }
    }
    Ok(())
}

pub(crate) async fn resolve_order_item_sku_mapping(
    pool: &PgPool,
    tenant_id: TenantId,
    shop_id: ShopId,
    platform: Platform,
    platform_sku_id: &str,
) -> anyhow::Result<Option<String>> {
    let mapped: Option<uuid::Uuid> = sqlx::query_scalar(
        r#"
        select sku_id
        from channel_sku_mappings
        where tenant_id = $1
          and shop_id = $2
          and platform = $3
          and platform_sku_id = $4
          and status = 'ACTIVE'
        limit 1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(shop_id))
    .bind(platform.to_string())
    .bind(platform_sku_id)
    .fetch_optional(pool)
    .await?;

    Ok(mapped.map(|id| id.to_string()))
}
