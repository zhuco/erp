use super::*;

pub(crate) async fn save_sales_shop_config(
    state: &AppState,
    tenant_id: TenantId,
    shop: Shop,
    config: SalesShopConfig,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    audit_action: &str,
) -> Result<ShopSalesConfigView, Response> {
    let settings_repo = SettingsRepo::new(state.db.pool());
    for (key, value_json) in config.to_settings() {
        let result = settings_repo
            .upsert_setting(SettingUpsert {
                tenant_id,
                scope_type: "SHOP".to_string(),
                scope_id: Some(uuid::Uuid::from(shop.shop_id)),
                key,
                value_json,
                expected_version: None,
            })
            .await;
        if let Err(err) = result {
            if err.to_string().contains("version conflict") {
                return Err(conflict("settings version conflict").into_response());
            }
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response());
        }
    }

    let sync_repo = SyncJobsRepo::new(state.db.pool());
    let _ = sync_repo
        .update_job_status(
            tenant_id,
            shop.shop_id,
            "order_sync",
            if config.auto_download_orders {
                "PENDING"
            } else {
                "DISABLED"
            },
            if config.auto_download_orders {
                None
            } else {
                Some("disabled by sales config".to_string())
            },
        )
        .await;
    let _ = sync_repo
        .update_job_status(
            tenant_id,
            shop.shop_id,
            "refund_sync",
            if config.enable_after_sales_download {
                "PENDING"
            } else {
                "DISABLED"
            },
            if config.enable_after_sales_download {
                None
            } else {
                Some("disabled by sales config".to_string())
            },
        )
        .await;
    let _ = sync_repo
        .update_job_status(
            tenant_id,
            shop.shop_id,
            "shipping_sync",
            if config.enable_logistics_upload {
                "PENDING"
            } else {
                "DISABLED"
            },
            if config.enable_logistics_upload {
                None
            } else {
                Some("disabled by sales config".to_string())
            },
        )
        .await;

    record_audit_log(
        state,
        ctx,
        req_ctx,
        audit_action,
        "shop",
        Some(shop.shop_id.to_string()),
        None,
        Some(serde_json::json!({
            "shop_id": shop.shop_id,
            "config": &config,
        })),
    )
    .await;

    Ok(sales_shop_config_view(shop, config))
}

pub(crate) async fn load_sales_config_for_shop(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    shop_id: ShopId,
) -> anyhow::Result<SalesShopConfig> {
    let mut configs = load_sales_configs_for_shops(pool, tenant_id, &[shop_id]).await?;
    Ok(configs.remove(&shop_id).unwrap_or_default())
}

pub(crate) async fn load_sales_configs_for_shops(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    shop_ids: &[ShopId],
) -> anyhow::Result<HashMap<ShopId, SalesShopConfig>> {
    let mut configs: HashMap<ShopId, SalesShopConfig> = HashMap::new();
    for shop_id in shop_ids {
        configs.insert(*shop_id, SalesShopConfig::default());
    }
    if shop_ids.is_empty() {
        return Ok(configs);
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        scope_id: uuid::Uuid,
        key: String,
        value_json: Value,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select scope_id, key, value_json
        from settings
        where tenant_id = $1
          and scope_type = 'SHOP'
          and scope_id = any($2)
          and key = any($3)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(
        shop_ids
            .iter()
            .map(|id| uuid::Uuid::from(*id))
            .collect::<Vec<_>>(),
    )
    .bind(
        sales_settings::SALES_CONFIG_KEYS
            .iter()
            .map(|key| key.to_string())
            .collect::<Vec<_>>(),
    )
    .fetch_all(pool)
    .await?;

    for row in rows {
        let shop_id = ShopId(row.scope_id);
        let entry = configs.entry(shop_id).or_default();
        entry.apply_setting(&row.key, &row.value_json);
    }

    Ok(configs)
}
