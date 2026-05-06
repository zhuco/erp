use super::*;

pub(super) async fn handle_sync_job_run(
    db: &Db,
    payload: SyncJobReplayPayload,
) -> anyhow::Result<()> {
    let jobs_repo = SyncJobsRepo::new(db.pool());
    let Some(job) = jobs_repo.get_job(payload.tenant_id, payload.job_id).await? else {
        warn!(
            tenant_id = %payload.tenant_id,
            job_id = %payload.job_id,
            "sync job replay ignored: job missing"
        );
        return Ok(());
    };

    let config = load_sales_config_for_shop(db.pool(), payload.tenant_id, job.shop_id).await?;
    let disable_reason = match job.job_type.as_str() {
        "order_sync" if !config.auto_download_orders => Some("sales auto download disabled"),
        "refund_sync" if !config.enable_after_sales_download => {
            Some("after-sales download disabled")
        }
        "shipping_sync" if !config.enable_logistics_upload => Some("logistics upload disabled"),
        _ => None,
    };
    if let Some(reason) = disable_reason {
        let _ = jobs_repo
            .update_job_status(
                payload.tenant_id,
                job.shop_id,
                &job.job_type,
                "DISABLED",
                Some(reason.to_string()),
            )
            .await?;
        return Ok(());
    }

    let _ = jobs_repo
        .update_job_result(
            payload.tenant_id,
            payload.job_id,
            "PENDING",
            job.cursor.clone(),
            job.watermark,
            Some(SYNC_JOB_FORCE_RUN_MARKER.to_string()),
        )
        .await?;
    Ok(())
}

pub(super) fn order_item_uuid(order_id: erp_types::OrderId, line_id: &str) -> uuid::Uuid {
    uuid::Uuid::parse_str(line_id)
        .unwrap_or_else(|_| uuid::Uuid::new_v5(&uuid::Uuid::from(order_id), line_id.as_bytes()))
}

pub(super) async fn apply_sku_mappings(
    pool: &PgPool,
    outbox_repo: &OutboxRepo<'_>,
    order: &mut OrderProjection,
) -> anyhow::Result<()> {
    for item in &mut order.items {
        let Some(platform_sku_id) = item.sku_id.clone() else {
            continue;
        };

        let mapped_sku_id = resolve_sku_mapping(
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

pub(super) async fn resolve_sku_mapping(
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

pub(super) fn sync_event_type(job_type: &str) -> String {
    job_type
        .split('_')
        .map(|part| part.to_uppercase())
        .collect::<Vec<_>>()
        .join("_")
}

pub(super) async fn load_sales_config_for_shop(
    pool: &PgPool,
    tenant_id: TenantId,
    shop_id: ShopId,
) -> anyhow::Result<SalesShopConfig> {
    #[derive(sqlx::FromRow)]
    struct Row {
        key: String,
        value_json: serde_json::Value,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select key, value_json
        from settings
        where tenant_id = $1
          and scope_type = 'SHOP'
          and scope_id = $2
          and key = any($3)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(shop_id))
    .bind(
        sales_settings::SALES_CONFIG_KEYS
            .iter()
            .map(|key| key.to_string())
            .collect::<Vec<_>>(),
    )
    .fetch_all(pool)
    .await?;

    let mut config = SalesShopConfig::default();
    for row in rows {
        config.apply_setting(&row.key, &row.value_json);
    }
    Ok(config)
}

pub(super) fn compute_sync_dedup_key(
    platform: &Platform,
    platform_order_id: &str,
    event_type: &str,
    version: &str,
    refund_fingerprint: Option<&str>,
) -> String {
    let base = match refund_fingerprint {
        Some(fingerprint) if !fingerprint.trim().is_empty() => {
            format!("{platform}:{platform_order_id}:{event_type}:{version}:{fingerprint}")
        }
        _ => format!("{platform}:{platform_order_id}:{event_type}:{version}"),
    };
    let mut hasher = Sha256::new();
    hasher.update(base.as_bytes());
    hex::encode(hasher.finalize())
}

pub(super) fn refund_sync_dedup_fingerprint(order: &OrderProjection) -> Option<String> {
    let mut tokens: Vec<String> = Vec::new();
    for item in &order.items {
        let case_id = item
            .after_sale_id
            .as_ref()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
            .unwrap_or_else(|| format!("LINE:{}", item.line_id));
        let token = format!(
            "{}:{}:{}:{}",
            case_id,
            item.refund_state,
            item.after_sale_state,
            item.refund_amount_cent.unwrap_or(0)
        );
        if token.ends_with(":NONE:NONE:0") && case_id.starts_with("LINE:") {
            continue;
        }
        tokens.push(token);
    }
    if tokens.is_empty() {
        return None;
    }
    tokens.sort();
    tokens.dedup();
    let joined = tokens.join("|");
    let mut hasher = Sha256::new();
    hasher.update(joined.as_bytes());
    Some(hex::encode(hasher.finalize()))
}

pub(super) fn annotate_mapping_version(
    order: &mut OrderProjection,
    mapping_version: Option<i32>,
) -> Option<i32> {
    let version = mapping_version?;
    let map = match order.raw.as_object_mut() {
        Some(map) => map,
        None => {
            order.raw = serde_json::Value::Object(serde_json::Map::new());
            order.raw.as_object_mut().expect("mapping raw object")
        }
    };
    map.insert(
        "applied_mapping_version".to_string(),
        serde_json::Value::Number(version.into()),
    );
    Some(version)
}

pub(super) fn normalize_text(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub(super) fn derive_platform_sku_code(sku: &erp_adapter::PlatformSku) -> String {
    let candidates = [
        sku.sku_code.as_ref(),
        sku.outer_sku_id.as_ref(),
        Some(&sku.platform_sku_id),
    ];
    for raw in candidates.into_iter().flatten() {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    sku.platform_sku_id.clone()
}

pub(super) async fn sync_platform_products(
    tenant_id: TenantId,
    platform: Platform,
    products: Vec<erp_adapter::PlatformProduct>,
    products_repo: &ProductsRepo<'_>,
    skus_repo: &SkusRepo<'_>,
) -> anyhow::Result<()> {
    for product in products {
        if product.skus.is_empty() {
            continue;
        }
        let mut product_id: Option<uuid::Uuid> = None;
        let mut product_updated = false;
        for sku in product.skus {
            let sku_code = derive_platform_sku_code(&sku);
            let existing = skus_repo
                .list_skus(
                    tenant_id,
                    SkuListFilters {
                        product_id: None,
                        sku_code: Some(sku_code.clone()),
                        barcode: None,
                        keyword: None,
                        is_active: None,
                        cursor: None,
                        limit: 1,
                    },
                )
                .await?;

            if let Some(existing_sku) = existing.items.first() {
                if product_id.is_none() {
                    product_id = Some(existing_sku.product_id);
                }
                if !product_updated {
                    let _ = products_repo
                        .update_product(
                            tenant_id,
                            existing_sku.product_id,
                            ProductUpdate {
                                product_code: None,
                                name: Some(product.name.clone()),
                                alias: None,
                                brand: normalize_text(product.brand.clone()),
                                category: normalize_text(product.category.clone()),
                                base_unit: None,
                                second_unit: None,
                                default_supplier_id: None,
                                default_warehouse_id: None,
                                primary_barcode: None,
                                product_tags: None,
                                remark: None,
                                warning_stock: None,
                                batch_enabled: None,
                                shelf_life_days: None,
                                forbid_sale_days: None,
                                serial_enabled: None,
                                serial_push_platform: None,
                                attrs: Some(product.attrs.clone()),
                                status: None,
                                allow_negative_stock: None,
                            },
                        )
                        .await?;
                    product_updated = true;
                }
                let _ = skus_repo
                    .update_sku(
                        tenant_id,
                        existing_sku.sku_id,
                        SkuUpdate {
                            sku_code: None,
                            barcode: normalize_text(sku.barcode.clone()),
                            name: normalize_text(sku.name.clone()),
                            note: None,
                            spec: sku.spec.clone(),
                            tags: if sku.tags.is_empty() {
                                None
                            } else {
                                Some(sku.tags.clone())
                            },
                            image_urls: if sku.image_urls.is_empty() {
                                None
                            } else {
                                Some(sku.image_urls.clone())
                            },
                            weight_gram: sku.weight_gram,
                            length_mm: sku.length_mm,
                            width_mm: sku.width_mm,
                            height_mm: sku.height_mm,
                            volume_cm3: sku.volume_cm3,
                            sku_kind: None,
                            cost_cent: None,
                            is_active: Some(true),
                            status: None,
                            batch_enabled: None,
                            serial_enabled: None,
                        },
                    )
                    .await?;
                continue;
            }

            if product_id.is_none() {
                let created = products_repo
                    .create_product(ProductCreate {
                        tenant_id,
                        product_code: None,
                        name: product.name.clone(),
                        alias: None,
                        brand: normalize_text(product.brand.clone()),
                        category: normalize_text(product.category.clone()),
                        base_unit: None,
                        second_unit: None,
                        default_supplier_id: None,
                        default_warehouse_id: None,
                        primary_barcode: None,
                        product_tags: Vec::new(),
                        remark: None,
                        warning_stock: serde_json::json!({}),
                        batch_enabled: false,
                        shelf_life_days: None,
                        forbid_sale_days: None,
                        serial_enabled: false,
                        serial_push_platform: false,
                        attrs: product.attrs.clone(),
                        allow_negative_stock: false,
                    })
                    .await?;
                product_id = Some(created.product_id);
                info!(
                    tenant_id = %tenant_id,
                    platform = %platform,
                    product_id = %created.product_id,
                    "platform product created"
                );
            }

            let _ = skus_repo
                .create_sku(SkuCreate {
                    tenant_id,
                    product_id: product_id.expect("product_id"),
                    sku_code,
                    barcode: normalize_text(sku.barcode.clone()),
                    name: normalize_text(sku.name.clone()),
                    note: None,
                    spec: sku.spec.unwrap_or_else(|| serde_json::json!({})),
                    tags: sku.tags.clone(),
                    image_urls: sku.image_urls.clone(),
                    weight_gram: sku.weight_gram,
                    length_mm: sku.length_mm,
                    width_mm: sku.width_mm,
                    height_mm: sku.height_mm,
                    volume_cm3: sku.volume_cm3,
                    sku_kind: "NORMAL".to_string(),
                    cost_cent: None,
                    is_active: true,
                    status: None,
                    batch_enabled: false,
                    serial_enabled: false,
                })
                .await?;
        }
    }
    Ok(())
}

pub(super) async fn run_sync_jobs_once(
    db: &Db,
    adapters: &AdapterRegistry,
    lookback: ChronoDuration,
    batch_limit: i64,
) -> anyhow::Result<()> {
    let jobs_repo = SyncJobsRepo::new(db.pool());
    let shops_repo = ShopsRepo::new(db.pool());
    let inbox_repo = InboxRepo::new(db.pool());
    let outbox_repo = OutboxRepo::new(db.pool());
    let mapping_rules_repo = MappingRulesRepo::new(db.pool());
    let products_repo = ProductsRepo::new(db.pool());
    let skus_repo = SkusRepo::new(db.pool());
    let runs_repo = SyncJobRunsRepo::new(db.pool());
    let mut config_cache: HashMap<ShopId, SalesShopConfig> = HashMap::new();

    let jobs = jobs_repo.list_jobs_for_run(batch_limit).await?;
    for job in jobs {
        let supported = matches!(
            job.job_type.as_str(),
            "order_sync" | "refund_sync" | "shipping_sync" | "product_sync"
        );
        if !supported {
            let _ = jobs_repo
                .update_job_result(
                    job.tenant_id,
                    job.job_id,
                    "FAILED",
                    job.cursor.clone(),
                    job.watermark,
                    Some("unsupported job_type".to_string()),
                )
                .await?;
            continue;
        }

        let config = if let Some(cfg) = config_cache.get(&job.shop_id) {
            cfg.clone()
        } else {
            let cfg = load_sales_config_for_shop(db.pool(), job.tenant_id, job.shop_id).await?;
            config_cache.insert(job.shop_id, cfg.clone());
            cfg
        };
        let disable_reason = match job.job_type.as_str() {
            "order_sync" if !config.auto_download_orders => Some("sales auto download disabled"),
            "refund_sync" if !config.enable_after_sales_download => {
                Some("after-sales download disabled")
            }
            "shipping_sync" if !config.enable_logistics_upload => Some("logistics upload disabled"),
            _ => None,
        };
        if let Some(reason) = disable_reason {
            let _ = jobs_repo
                .update_job_status(
                    job.tenant_id,
                    job.shop_id,
                    &job.job_type,
                    "DISABLED",
                    Some(reason.to_string()),
                )
                .await?;
            continue;
        }

        let force_run = job.last_error.trim() == SYNC_JOB_FORCE_RUN_MARKER;
        if !force_run && job.job_type == "order_sync" && config.download_interval_minutes > 0 {
            let interval = ChronoDuration::minutes(config.download_interval_minutes.max(1));
            if job.updated_at + interval > Utc::now() {
                continue;
            }
        }

        let sync_flag = match job.job_type.as_str() {
            "order_sync" => Some(FeatureFlagKey::OrderSync),
            "refund_sync" => Some(FeatureFlagKey::AfterSalesSync),
            "shipping_sync" => Some(FeatureFlagKey::ShippingWrite),
            "product_sync" => None,
            _ => Some(FeatureFlagKey::OrderSync),
        };
        if let Some(sync_flag) = sync_flag {
            let sync_enabled =
                feature_flag_enabled(db, job.tenant_id, Some(job.shop_id), sync_flag).await?;
            if !sync_enabled {
                warn!(
                    tenant_id = %job.tenant_id,
                    shop_id = %job.shop_id,
                    job_type = %job.job_type,
                    "feature disabled: sync job"
                );
                continue;
            }
        }
        if job.job_type == "order_sync" && config.auto_merge_non_shipping_time {
            let merge_enabled = feature_flag_enabled(
                db,
                job.tenant_id,
                Some(job.shop_id),
                FeatureFlagKey::OrderMerge,
            )
            .await?;
            if !merge_enabled {
                warn!(
                    tenant_id = %job.tenant_id,
                    shop_id = %job.shop_id,
                    job_type = %job.job_type,
                    "feature disabled: order merge"
                );
            }
        }

        let Some(job) = jobs_repo
            .try_mark_running(job.tenant_id, job.job_id)
            .await?
        else {
            continue;
        };
        let shop = match shops_repo.get_shop_auth(job.shop_id).await {
            Ok(shop) => shop,
            Err(err) => {
                let _ = jobs_repo
                    .update_job_result(
                        job.tenant_id,
                        job.job_id,
                        "FAILED",
                        job.cursor.clone(),
                        job.watermark,
                        Some(err.to_string()),
                    )
                    .await?;
                continue;
            }
        };
        let adapter = adapters.platform_adapter(shop.platform);
        let cursor = SyncCursor {
            cursor: job.cursor.clone(),
            last_time: job.watermark.or_else(|| Some(Utc::now() - lookback)),
        };

        if job.job_type == "product_sync" {
            let started_at = Utc::now();
            let result = adapter.pull_products(&shop, cursor).await;
            let finished_at = Utc::now();
            let duration_ms = (finished_at - started_at).num_milliseconds().max(0);
            match result {
                Ok(res) => {
                    let item_count = res.products.len() as i32;
                    if let Err(err) = sync_platform_products(
                        job.tenant_id,
                        shop.platform,
                        res.products,
                        &products_repo,
                        &skus_repo,
                    )
                    .await
                    {
                        let _ = jobs_repo
                            .update_job_result(
                                job.tenant_id,
                                job.job_id,
                                "FAILED",
                                job.cursor.clone(),
                                job.watermark,
                                Some(err.to_string()),
                            )
                            .await?;
                        let _ = runs_repo
                            .insert_run(SyncJobRunCreate {
                                tenant_id: job.tenant_id,
                                job_id: job.job_id,
                                shop_id: job.shop_id,
                                job_type: job.job_type.clone(),
                                status: "FAILED".to_string(),
                                item_count: 0,
                                duration_ms,
                                error: err.to_string(),
                                started_at,
                                finished_at,
                            })
                            .await;

                        let failure = anyhow::anyhow!(err);
                        let classification = dlq::classify_error(&failure);
                        let env = OutboxEnvelope {
                            tenant_id: job.tenant_id,
                            outbox_id: job.job_id,
                            event_type: "SYNC_JOB_RUN".to_string(),
                            payload: serde_json::json!({
                                "tenant_id": job.tenant_id,
                                "job_id": job.job_id,
                                "shop_id": job.shop_id,
                                "job_type": job.job_type,
                            }),
                            produced_at: Utc::now(),
                            trace_id: None,
                        };
                        let _ = dlq::insert_dead_letter(
                            db.pool(),
                            &env,
                            &failure.to_string(),
                            1,
                            classification,
                            dlq::NEXT_ACTION_MANUAL_REPLAY,
                        )
                        .await?;
                        continue;
                    }

                    let cursor = res.next_cursor.or(job.cursor.clone());
                    let last_time = res.latest_time.or(job.watermark);
                    let _ = jobs_repo
                        .update_job_result(
                            job.tenant_id,
                            job.job_id,
                            "SUCCESS",
                            cursor,
                            last_time,
                            None,
                        )
                        .await?;
                    let _ = runs_repo
                        .insert_run(SyncJobRunCreate {
                            tenant_id: job.tenant_id,
                            job_id: job.job_id,
                            shop_id: job.shop_id,
                            job_type: job.job_type.clone(),
                            status: "SUCCESS".to_string(),
                            item_count,
                            duration_ms,
                            error: String::new(),
                            started_at,
                            finished_at,
                        })
                        .await;
                }
                Err(err) => {
                    let _ = jobs_repo
                        .update_job_result(
                            job.tenant_id,
                            job.job_id,
                            "FAILED",
                            job.cursor.clone(),
                            job.watermark,
                            Some(err.to_string()),
                        )
                        .await?;
                    let _ = runs_repo
                        .insert_run(SyncJobRunCreate {
                            tenant_id: job.tenant_id,
                            job_id: job.job_id,
                            shop_id: job.shop_id,
                            job_type: job.job_type.clone(),
                            status: "FAILED".to_string(),
                            item_count: 0,
                            duration_ms,
                            error: err.to_string(),
                            started_at,
                            finished_at,
                        })
                        .await;

                    let failure = anyhow::anyhow!(err);
                    let classification = dlq::classify_error(&failure);
                    let env = OutboxEnvelope {
                        tenant_id: job.tenant_id,
                        outbox_id: job.job_id,
                        event_type: "SYNC_JOB_RUN".to_string(),
                        payload: serde_json::json!({
                            "tenant_id": job.tenant_id,
                            "job_id": job.job_id,
                            "shop_id": job.shop_id,
                            "job_type": job.job_type,
                        }),
                        produced_at: Utc::now(),
                        trace_id: None,
                    };
                    let _ = dlq::insert_dead_letter(
                        db.pool(),
                        &env,
                        &failure.to_string(),
                        1,
                        classification,
                        dlq::NEXT_ACTION_MANUAL_REPLAY,
                    )
                    .await?;
                }
            }
            continue;
        }

        let mapping_version = mapping_rules_repo
            .latest_enabled_version(job.tenant_id, shop.platform, "order")
            .await?;
        let started_at = Utc::now();
        let result = if job.job_type == "refund_sync" {
            adapter
                .pull_refunds(&shop, cursor)
                .await
                .map(|res| erp_adapter::PullOrdersResult {
                    orders: res.orders,
                    next_cursor: res.next_cursor,
                    latest_time: res.latest_time,
                })
        } else {
            adapter.pull_orders(&shop, cursor).await
        };
        let finished_at = Utc::now();
        let duration_ms = (finished_at - started_at).num_milliseconds().max(0);
        match result {
            Ok(res) => {
                let event_type = sync_event_type(&job.job_type);
                let mut latest_time = job.watermark;
                let orders = res.orders;
                let item_count = orders.len() as i32;
                for mut order in orders {
                    annotate_mapping_version(&mut order, mapping_version);
                    let version_time = order.platform_updated_at.unwrap_or(order.order_created_at);
                    let version = version_time.timestamp().to_string();
                    let refund_fingerprint = if job.job_type == "refund_sync" {
                        refund_sync_dedup_fingerprint(&order)
                    } else {
                        None
                    };
                    let dedup_key = compute_sync_dedup_key(
                        &order.platform,
                        &order.platform_order_id,
                        &event_type,
                        &version,
                        refund_fingerprint.as_deref(),
                    );
                    let headers = serde_json::json!({
                        "source": "sync_job",
                        "job_id": job.job_id,
                        "job_type": job.job_type,
                        "platform_order_id": order.platform_order_id,
                        "version": version,
                    });
                    let body = serde_json::to_vec(&order.raw)?;
                    let received_at = Utc::now();
                    let is_first = inbox_repo
                        .claim_dedup_key(order.tenant_id, &dedup_key)
                        .await?;
                    let raw_id = inbox_repo
                        .insert_raw_payload(
                            order.tenant_id,
                            order.platform,
                            order.shop_id,
                            &headers,
                            &body,
                            received_at,
                        )
                        .await?;
                    let status = if is_first { "RECEIVED" } else { "DUPLICATE" };
                    let inbox_event_id = inbox_repo
                        .insert_inbox_event(
                            order.tenant_id,
                            order.platform,
                            order.shop_id,
                            &dedup_key,
                            raw_id,
                            received_at,
                            status,
                            "PASSED",
                            None,
                        )
                        .await?;

                    if is_first {
                        let payload = serde_json::to_value(InboxReceivedPayload {
                            tenant_id: order.tenant_id,
                            platform: order.platform,
                            shop_id: order.shop_id,
                            inbox_event_id,
                        })?;
                        outbox_repo
                            .enqueue(order.tenant_id, "INBOX_EVENT_RECEIVED", &payload)
                            .await?;
                    }

                    if latest_time.map(|t| version_time > t).unwrap_or(true) {
                        latest_time = Some(version_time);
                    }
                }

                let cursor = res.next_cursor.or(job.cursor.clone());
                let last_time = res.latest_time.or(latest_time);
                let _ = jobs_repo
                    .update_job_result(
                        job.tenant_id,
                        job.job_id,
                        "SUCCESS",
                        cursor,
                        last_time,
                        None,
                    )
                    .await?;
                let _ = runs_repo
                    .insert_run(SyncJobRunCreate {
                        tenant_id: job.tenant_id,
                        job_id: job.job_id,
                        shop_id: job.shop_id,
                        job_type: job.job_type.clone(),
                        status: "SUCCESS".to_string(),
                        item_count,
                        duration_ms,
                        error: String::new(),
                        started_at,
                        finished_at,
                    })
                    .await;
            }
            Err(err) => {
                let _ = jobs_repo
                    .update_job_result(
                        job.tenant_id,
                        job.job_id,
                        "FAILED",
                        job.cursor.clone(),
                        job.watermark,
                        Some(err.to_string()),
                    )
                    .await?;
                let _ = runs_repo
                    .insert_run(SyncJobRunCreate {
                        tenant_id: job.tenant_id,
                        job_id: job.job_id,
                        shop_id: job.shop_id,
                        job_type: job.job_type.clone(),
                        status: "FAILED".to_string(),
                        item_count: 0,
                        duration_ms,
                        error: err.to_string(),
                        started_at,
                        finished_at,
                    })
                    .await;

                let failure = anyhow::anyhow!(err);
                let classification = dlq::classify_error(&failure);
                let env = OutboxEnvelope {
                    tenant_id: job.tenant_id,
                    outbox_id: job.job_id,
                    event_type: "SYNC_JOB_RUN".to_string(),
                    payload: serde_json::json!({
                        "tenant_id": job.tenant_id,
                        "job_id": job.job_id,
                        "shop_id": job.shop_id,
                        "job_type": job.job_type,
                    }),
                    produced_at: Utc::now(),
                    trace_id: None,
                };
                let _ = dlq::insert_dead_letter(
                    db.pool(),
                    &env,
                    &failure.to_string(),
                    1,
                    classification,
                    dlq::NEXT_ACTION_MANUAL_REPLAY,
                )
                .await?;
            }
        }
    }

    Ok(())
}
