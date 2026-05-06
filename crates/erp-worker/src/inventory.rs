use super::*;

pub(super) async fn handle_inventory_changed(
    db: &Db,
    payload: InventoryChangedPayload,
    debounce: ChronoDuration,
) -> anyhow::Result<()> {
    if read_only_enabled(db, payload.tenant_id).await? {
        warn!(
            tenant_id = %payload.tenant_id,
            sku_id = %payload.sku_id,
            "read-only mode active; skipping inventory sync"
        );
        return Ok(());
    }

    #[derive(sqlx::FromRow)]
    struct MappingRow {
        shop_id: uuid::Uuid,
    }

    let mappings: Vec<MappingRow> = sqlx::query_as(
        r#"
        select m.shop_id
        from channel_sku_mappings m
        join shops s on s.shop_id = m.shop_id
        where m.tenant_id = $1
          and m.sku_id = $2
          and m.status = 'ACTIVE'
          and s.status = 'ACTIVE'
        "#,
    )
    .bind(uuid::Uuid::from(payload.tenant_id))
    .bind(payload.sku_id)
    .fetch_all(db.pool())
    .await?;

    if mappings.is_empty() {
        return Ok(());
    }

    let sync_repo = StockSyncJobsRepo::new(db.pool());
    let next_retry_at = Utc::now() + debounce;
    let mut config_cache: HashMap<ShopId, SalesShopConfig> = HashMap::new();

    for mapping in mappings {
        let shop_id = ShopId(mapping.shop_id);
        let config = if let Some(cfg) = config_cache.get(&shop_id) {
            cfg.clone()
        } else {
            let cfg = match load_sales_config_for_shop(db.pool(), payload.tenant_id, shop_id).await
            {
                Ok(v) => v,
                Err(err) => {
                    warn!(
                        tenant_id = %payload.tenant_id,
                        shop_id = %shop_id,
                        error = %err,
                        "failed to load sales config for stock sync; fallback to defaults"
                    );
                    SalesShopConfig::default()
                }
            };
            config_cache.insert(shop_id, cfg.clone());
            cfg
        };
        if !config.enable_stock_sync {
            warn!(
                tenant_id = %payload.tenant_id,
                shop_id = %shop_id,
                "stock sync disabled by sales config"
            );
            continue;
        }
        let sync_enabled = feature_flag_enabled(
            db,
            payload.tenant_id,
            Some(shop_id),
            FeatureFlagKey::InventorySync,
        )
        .await?;
        if !sync_enabled {
            warn!(
                tenant_id = %payload.tenant_id,
                shop_id = %shop_id,
                "feature disabled: inventory sync"
            );
            continue;
        }
        let _job = sync_repo
            .upsert_pending_job(erp_repo::stock_sync_jobs::StockSyncJobUpsert {
                tenant_id: payload.tenant_id,
                shop_id,
                sku_id: payload.sku_id,
                warehouse_id: payload.warehouse_id,
                target_qty: payload.available,
                next_retry_at,
            })
            .await?;
    }

    Ok(())
}

pub(super) async fn run_stock_sync_jobs_once(
    db: &Db,
    adapters: &AdapterRegistry,
    batch_limit: i64,
    retry_policy: &dlq::RetryPolicy,
) -> anyhow::Result<()> {
    let jobs_repo = StockSyncJobsRepo::new(db.pool());
    let events_repo = StockSyncJobEventsRepo::new(db.pool());
    let shops_repo = ShopsRepo::new(db.pool());
    let mut config_cache: HashMap<ShopId, SalesShopConfig> = HashMap::new();

    #[derive(sqlx::FromRow)]
    struct MappingRow {
        platform: String,
        platform_sku_id: String,
    }

    let jobs = jobs_repo.list_jobs_for_run(batch_limit).await?;
    for job in jobs {
        let now = Utc::now();
        let Some(job) = jobs_repo
            .try_mark_sending(job.tenant_id, job.job_id, job.updated_at, now)
            .await?
        else {
            continue;
        };

        let attempt = job.attempts.saturating_add(1);
        let config = if let Some(cfg) = config_cache.get(&job.shop_id) {
            cfg.clone()
        } else {
            let cfg = match load_sales_config_for_shop(db.pool(), job.tenant_id, job.shop_id).await
            {
                Ok(v) => v,
                Err(err) => {
                    warn!(
                        tenant_id = %job.tenant_id,
                        shop_id = %job.shop_id,
                        error = %err,
                        "failed to load sales config for stock sync; fallback to defaults"
                    );
                    SalesShopConfig::default()
                }
            };
            config_cache.insert(job.shop_id, cfg.clone());
            cfg
        };
        if !config.enable_stock_sync {
            let error = "stock sync disabled by sales config".to_string();
            let _ = jobs_repo
                .mark_failed(job.tenant_id, job.job_id, error.clone(), "DLQ", now, 0)
                .await?;
            let payload = serde_json::json!({
                "error": error,
                "classification": "DISABLED",
            });
            let _ = events_repo
                .insert_event(erp_repo::stock_sync_job_events::StockSyncJobEventCreate {
                    tenant_id: job.tenant_id,
                    job_id: job.job_id,
                    shop_id: job.shop_id,
                    sku_id: job.sku_id,
                    warehouse_id: job.warehouse_id,
                    status: "ERROR".to_string(),
                    attempt,
                    payload,
                })
                .await?;
            continue;
        }
        let sync_enabled = feature_flag_enabled(
            db,
            job.tenant_id,
            Some(job.shop_id),
            FeatureFlagKey::InventorySync,
        )
        .await?;
        if !sync_enabled {
            let error = "feature disabled: inventory sync".to_string();
            let _ = jobs_repo
                .mark_failed(job.tenant_id, job.job_id, error.clone(), "DLQ", now, 0)
                .await?;
            let payload = serde_json::json!({
                "error": error,
                "classification": "DISABLED",
            });
            let _ = events_repo
                .insert_event(erp_repo::stock_sync_job_events::StockSyncJobEventCreate {
                    tenant_id: job.tenant_id,
                    job_id: job.job_id,
                    shop_id: job.shop_id,
                    sku_id: job.sku_id,
                    warehouse_id: job.warehouse_id,
                    status: "ERROR".to_string(),
                    attempt,
                    payload,
                })
                .await?;
            continue;
        }
        let mapping: Option<MappingRow> = sqlx::query_as(
            r#"
            select m.platform, m.platform_sku_id
            from channel_sku_mappings m
            join shops s on s.shop_id = m.shop_id
            where m.tenant_id = $1
              and m.shop_id = $2
              and m.sku_id = $3
              and m.status = 'ACTIVE'
              and s.status = 'ACTIVE'
            "#,
        )
        .bind(uuid::Uuid::from(job.tenant_id))
        .bind(uuid::Uuid::from(job.shop_id))
        .bind(job.sku_id)
        .fetch_optional(db.pool())
        .await?;

        let Some(mapping) = mapping else {
            let error = "active channel mapping not found".to_string();
            let _ = jobs_repo
                .mark_failed(job.tenant_id, job.job_id, error.clone(), "DLQ", now, 0)
                .await?;
            let payload = serde_json::json!({
                "error": error,
                "classification": "PERMANENT",
            });
            let _ = events_repo
                .insert_event(erp_repo::stock_sync_job_events::StockSyncJobEventCreate {
                    tenant_id: job.tenant_id,
                    job_id: job.job_id,
                    shop_id: job.shop_id,
                    sku_id: job.sku_id,
                    warehouse_id: job.warehouse_id,
                    status: "ERROR".to_string(),
                    attempt,
                    payload,
                })
                .await?;
            continue;
        };

        let platform = match Platform::from_str(&mapping.platform) {
            Ok(platform) => platform,
            Err(err) => {
                let _ = jobs_repo
                    .mark_failed(job.tenant_id, job.job_id, err.to_string(), "DLQ", now, 0)
                    .await?;
                let payload = serde_json::json!({
                    "error": err.to_string(),
                    "classification": "PERMANENT",
                });
                let _ = events_repo
                    .insert_event(erp_repo::stock_sync_job_events::StockSyncJobEventCreate {
                        tenant_id: job.tenant_id,
                        job_id: job.job_id,
                        shop_id: job.shop_id,
                        sku_id: job.sku_id,
                        warehouse_id: job.warehouse_id,
                        status: "ERROR".to_string(),
                        attempt,
                        payload,
                    })
                    .await?;
                continue;
            }
        };

        let shop = match shops_repo.get_shop_auth(job.shop_id).await {
            Ok(shop) => shop,
            Err(err) => {
                let _ = jobs_repo
                    .mark_failed(job.tenant_id, job.job_id, err.to_string(), "DLQ", now, 0)
                    .await?;
                let payload = serde_json::json!({
                    "error": err.to_string(),
                    "classification": "PERMANENT",
                });
                let _ = events_repo
                    .insert_event(erp_repo::stock_sync_job_events::StockSyncJobEventCreate {
                        tenant_id: job.tenant_id,
                        job_id: job.job_id,
                        shop_id: job.shop_id,
                        sku_id: job.sku_id,
                        warehouse_id: job.warehouse_id,
                        status: "ERROR".to_string(),
                        attempt,
                        payload,
                    })
                    .await?;
                continue;
            }
        };

        let request = SyncStockRequest {
            platform_sku_id: mapping.platform_sku_id,
            warehouse_id: job.warehouse_id,
            target_qty: job.target_qty,
        };
        let request_payload = serde_json::json!({
            "platform": platform.to_string(),
            "platform_sku_id": request.platform_sku_id,
            "warehouse_id": request.warehouse_id,
            "target_qty": request.target_qty,
        });
        let _ = events_repo
            .insert_event(erp_repo::stock_sync_job_events::StockSyncJobEventCreate {
                tenant_id: job.tenant_id,
                job_id: job.job_id,
                shop_id: job.shop_id,
                sku_id: job.sku_id,
                warehouse_id: job.warehouse_id,
                status: "REQUEST".to_string(),
                attempt,
                payload: request_payload,
            })
            .await?;

        let adapter = adapters.platform_adapter(platform);
        let started = std::time::Instant::now();
        match adapter.sync_stock(&shop, request).await {
            Ok(response) => {
                let duration_ms = started
                    .elapsed()
                    .as_millis()
                    .min(i64::from(i32::MAX) as u128) as i32;
                let _ = jobs_repo
                    .mark_sent(job.tenant_id, job.job_id, duration_ms)
                    .await?;
                let payload = serde_json::json!({
                    "target_qty": response.target_qty,
                    "duration_ms": duration_ms,
                });
                let _ = events_repo
                    .insert_event(erp_repo::stock_sync_job_events::StockSyncJobEventCreate {
                        tenant_id: job.tenant_id,
                        job_id: job.job_id,
                        shop_id: job.shop_id,
                        sku_id: job.sku_id,
                        warehouse_id: job.warehouse_id,
                        status: "RESPONSE".to_string(),
                        attempt,
                        payload,
                    })
                    .await?;
            }
            Err(err) => {
                let err = anyhow::anyhow!(err);
                let classification = dlq::classify_error(&err);
                let duration_ms = started
                    .elapsed()
                    .as_millis()
                    .min(i64::from(i32::MAX) as u128) as i32;
                let (status, next_retry_at) = if classification == dlq::FailureClass::Transient
                    && (attempt as u32) <= retry_policy.max_attempts
                {
                    let delay = retry_policy.next_delay(attempt as u32);
                    (
                        "FAILED",
                        Utc::now() + ChronoDuration::milliseconds(delay.as_millis() as i64),
                    )
                } else {
                    ("DLQ", Utc::now())
                };

                let _ = jobs_repo
                    .mark_failed(
                        job.tenant_id,
                        job.job_id,
                        err.to_string(),
                        status,
                        next_retry_at,
                        duration_ms,
                    )
                    .await?;
                let payload = serde_json::json!({
                    "error": err.to_string(),
                    "classification": classification.as_str(),
                    "next_action": status,
                    "duration_ms": duration_ms,
                });
                let _ = events_repo
                    .insert_event(erp_repo::stock_sync_job_events::StockSyncJobEventCreate {
                        tenant_id: job.tenant_id,
                        job_id: job.job_id,
                        shop_id: job.shop_id,
                        sku_id: job.sku_id,
                        warehouse_id: job.warehouse_id,
                        status: "ERROR".to_string(),
                        attempt,
                        payload,
                    })
                    .await?;
                warn!(
                    tenant_id = %job.tenant_id,
                    shop_id = %job.shop_id,
                    sku_id = %job.sku_id,
                    error = %err,
                    "inventory sync failed"
                );
            }
        }
    }

    Ok(())
}

pub(super) const WMS_WORKFLOW_CONFIG_KEY: &str = "wms.workflow.config";
