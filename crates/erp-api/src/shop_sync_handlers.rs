use super::*;

pub(crate) async fn list_shops(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ShopListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.read"]) {
        return resp.into_response();
    }

    let platform = match query.platform.as_deref() {
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                match Platform::from_str(trimmed) {
                    Ok(v) => Some(v),
                    Err(_) => return bad_request("bad platform").into_response(),
                }
            }
        }
        None => None,
    };
    let status = query
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let shop_ids = shop_scope_filter(&ctx);

    let repo = ShopsRepo::new(state.db.pool());
    let result = match repo
        .list_shops(
            tenant_id,
            RepoShopListFilters {
                platform,
                status,
                shop_ids,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let shop_ids: Vec<ShopId> = result.items.iter().map(|shop| shop.shop_id).collect();
    let tokens_repo = PlatformTokensRepo::new(state.db.pool());
    let tokens = match tokens_repo.list_tokens(tenant_id, &shop_ids).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let token_map: HashMap<ShopId, PlatformToken> = tokens
        .into_iter()
        .map(|token| (token.shop_id, token))
        .collect();

    let items = result
        .items
        .into_iter()
        .map(|shop| {
            let token = token_map.get(&shop.shop_id);
            shop_view(shop, token)
        })
        .collect();

    (
        StatusCode::OK,
        Json(ShopListResponse {
            items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn list_system_integration_shops_overview(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "settings.read") {
        return resp.into_response();
    }

    let repo = ShopsRepo::new(state.db.pool());
    let result = match repo
        .list_shops(
            tenant_id,
            RepoShopListFilters {
                platform: None,
                status: None,
                shop_ids: shop_scope_filter(&ctx),
                offset: 0,
                limit: 500,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let shop_ids: Vec<ShopId> = result.items.iter().map(|shop| shop.shop_id).collect();
    let tokens_repo = PlatformTokensRepo::new(state.db.pool());
    let tokens = match tokens_repo.list_tokens(tenant_id, &shop_ids).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let token_map: HashMap<ShopId, PlatformToken> = tokens
        .into_iter()
        .map(|token| (token.shop_id, token))
        .collect();

    let sync_repo = SyncJobsRepo::new(state.db.pool());
    let job_limit = (shop_ids.len() as i64 * 8).clamp(50, 4_000);
    let jobs = match sync_repo
        .list_jobs(
            tenant_id,
            RepoSyncJobListFilters {
                shop_id: None,
                shop_ids: Some(shop_ids.clone()),
                job_type: None,
                status: None,
                offset: 0,
                limit: job_limit,
            },
        )
        .await
    {
        Ok(v) => v.items,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let mut jobs_by_shop: HashMap<ShopId, Vec<SyncJob>> = HashMap::new();
    for job in jobs {
        jobs_by_shop.entry(job.shop_id).or_default().push(job);
    }

    let sales_configs =
        match load_sales_configs_for_shops(state.db.pool(), tenant_id, &shop_ids).await {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
    let credentials_map =
        match load_shop_credentials_for_shops(state.db.pool(), tenant_id, &shop_ids).await {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
    let webhook_health_map =
        match load_shop_webhook_health_for_shops(state.db.pool(), tenant_id, &shop_ids).await {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };

    let items = result
        .items
        .into_iter()
        .map(|shop| {
            let token = token_map.get(&shop.shop_id);
            let token_health = token.map(shop_token_status);
            let oauth_status = derive_shop_oauth_status(&shop, token);
            let sales_config = sales_configs
                .get(&shop.shop_id)
                .cloned()
                .unwrap_or_default();
            let webhook_mode = credentials_map.get(&shop.shop_id).and_then(|credentials| {
                json_string_at_pointer(credentials, "/webhook_verification/mode")
            });
            let webhook_health = webhook_health_map
                .get(&shop.shop_id)
                .cloned()
                .unwrap_or_else(|| {
                    derive_webhook_health_summary(None, None, 0, 0, 0, 0, 0, None, None)
                });
            let shop_jobs = jobs_by_shop.remove(&shop.shop_id).unwrap_or_default();
            let last_sync_at = shop_jobs.iter().filter_map(|job| job.last_success_at).max();
            let failed_job_count = shop_jobs
                .iter()
                .filter(|job| job.status.eq_ignore_ascii_case("FAILED"))
                .count() as i64;
            let enabled_job_count = shop_jobs
                .iter()
                .filter(|job| shop_sync_job_enabled(job, &sales_config))
                .count() as i64;

            let mut alert_messages = Vec::new();
            if oauth_status == "FAILED" {
                alert_messages.push("授权状态异常".to_string());
            }
            if let Some(token_health) = token_health.as_ref() {
                if token_health.is_expired {
                    alert_messages.push("Token 已过期".to_string());
                } else if token_health.is_expiring_soon {
                    alert_messages.push("Token 即将过期".to_string());
                }
                if token_health.refresh_fail_count > 0 && !token_health.last_error.trim().is_empty()
                {
                    alert_messages.push("Token 刷新异常".to_string());
                }
            }
            push_webhook_alert_messages(&mut alert_messages, &webhook_health);
            for job in &shop_jobs {
                if job.status.eq_ignore_ascii_case("FAILED") {
                    alert_messages.push(format!("{} 鍚屾失败", job.job_type));
                }
            }

            let integration_status = derive_shop_integration_status(
                &shop,
                &oauth_status,
                token_health.as_ref(),
                failed_job_count,
                &webhook_health,
            );
            let warning_count = alert_messages.len() as i64;
            let jobs = shop_jobs
                .into_iter()
                .map(|job| {
                    let enabled = shop_sync_job_enabled(&job, &sales_config);
                    let last_error = non_empty(job.last_error.clone());
                    ShopIntegrationOverviewJob {
                        job_type: job.job_type,
                        status: job.status,
                        enabled,
                        last_error,
                        last_success_at: job.last_success_at,
                    }
                })
                .collect();

            ShopIntegrationOverviewItem {
                shop_id: shop.shop_id,
                platform: shop.platform,
                platform_shop_id: shop.platform_shop_id,
                name: shop.name,
                shop_status: shop.status,
                oauth_status,
                integration_status,
                webhook_mode,
                webhook_status: webhook_health.webhook_status,
                webhook_sla_status: webhook_health.webhook_sla_status,
                last_webhook_received_at: webhook_health.last_webhook_received_at,
                last_webhook_processed_at: webhook_health.last_webhook_processed_at,
                webhook_failed_count: webhook_health.webhook_failed_count,
                webhook_verify_failed_count: webhook_health.webhook_verify_failed_count,
                webhook_pending_count: webhook_health.webhook_pending_count,
                webhook_retrying_count: webhook_health.webhook_retrying_count,
                webhook_dlq_count: webhook_health.webhook_dlq_count,
                webhook_next_retry_at: webhook_health.webhook_next_retry_at,
                token_status: token_health
                    .as_ref()
                    .map(|status| status.token_status.clone()),
                token_expires_at: token_health.as_ref().and_then(|status| status.expires_at),
                last_sync_at,
                enabled_job_count,
                failed_job_count,
                warning_count,
                alert_messages,
                jobs,
            }
        })
        .collect();

    (
        StatusCode::OK,
        Json(ShopIntegrationOverviewResponse {
            items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn list_sales_shop_configs(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ShopSalesConfigListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.read"]) {
        return resp.into_response();
    }

    let platform = match query.platform.as_deref() {
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                match Platform::from_str(trimmed) {
                    Ok(v) => Some(v),
                    Err(_) => return bad_request("bad platform").into_response(),
                }
            }
        }
        None => None,
    };
    let status = query
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    let shop_id = match query.shop_id.as_deref() {
        Some(raw) => match ShopId::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    if let Some(shop_id) = shop_id {
        if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
            return resp.into_response();
        }
    }
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let shop_ids = shop_scope_filter(&ctx);
    if let (Some(shop_id), Some(allowed)) = (shop_id, &shop_ids) {
        if !allowed.contains(&shop_id) {
            return forbidden("shop access denied").into_response();
        }
    }

    let repo = ShopsRepo::new(state.db.pool());
    let result = match repo
        .list_shops(
            tenant_id,
            RepoShopListFilters {
                platform,
                status,
                shop_ids: shop_id.map(|id| vec![id]).or(shop_ids),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let shop_list: Vec<ShopId> = result.items.iter().map(|shop| shop.shop_id).collect();
    let configs = match load_sales_configs_for_shops(state.db.pool(), tenant_id, &shop_list).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items = result
        .items
        .into_iter()
        .map(|shop| {
            let config = configs.get(&shop.shop_id).cloned().unwrap_or_default();
            sales_shop_config_view(shop, config)
        })
        .collect();

    (
        StatusCode::OK,
        Json(ShopSalesConfigListResponse {
            items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn get_sales_shop_config(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.read"]) {
        return resp.into_response();
    }

    let repo = ShopsRepo::new(state.db.pool());
    let shop = match repo.get_shop(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let config = match load_sales_config_for_shop(state.db.pool(), tenant_id, shop_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(sales_shop_config_view(shop, config))).into_response()
}

pub(crate) async fn update_sales_shop_config(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ShopSalesConfigRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.read"]) {
        return resp.into_response();
    }

    if req.download_interval_minutes <= 0 {
        return bad_request("download_interval_minutes must be positive").into_response();
    }
    let allowed_intervals = [5_i64, 10, 15, 30, 60];
    if !allowed_intervals.contains(&req.download_interval_minutes) {
        return bad_request("download_interval_minutes must be 5/10/15/30/60").into_response();
    }
    if req.auto_merge_non_shipping_time
        && (req.non_shipping_time_start.is_none() || req.non_shipping_time_end.is_none())
    {
        return bad_request("non_shipping_time_start/end required").into_response();
    }
    if let Some(start) = req.non_shipping_time_start.as_deref() {
        if chrono::NaiveTime::parse_from_str(start, "%H:%M").is_err() {
            return bad_request("bad non_shipping_time_start").into_response();
        }
    }
    if let Some(end) = req.non_shipping_time_end.as_deref() {
        if chrono::NaiveTime::parse_from_str(end, "%H:%M").is_err() {
            return bad_request("bad non_shipping_time_end").into_response();
        }
    }

    let repo = ShopsRepo::new(state.db.pool());
    let shop = match repo.get_shop(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let config = SalesShopConfig {
        auto_download_orders: req.auto_download_orders,
        auto_to_review: req.auto_to_review,
        download_interval_minutes: req.download_interval_minutes,
        auto_merge_non_shipping_time: req.auto_merge_non_shipping_time,
        non_shipping_time_start: req.non_shipping_time_start,
        non_shipping_time_end: req.non_shipping_time_end,
        enable_stock_sync: req.enable_stock_sync,
        enable_after_sales_download: req.enable_after_sales_download,
        enable_logistics_upload: req.enable_logistics_upload,
    };

    let settings_repo = SettingsRepo::new(state.db.pool());
    for (key, value_json) in config.to_settings() {
        let result = settings_repo
            .upsert_setting(SettingUpsert {
                tenant_id,
                scope_type: "SHOP".to_string(),
                scope_id: Some(uuid::Uuid::from(shop_id)),
                key,
                value_json,
                expected_version: None,
            })
            .await;
        if let Err(err) = result {
            if err.to_string().contains("version conflict") {
                return conflict("settings version conflict").into_response();
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
        }
    }

    let sync_repo = SyncJobsRepo::new(state.db.pool());
    let _ = sync_repo
        .update_job_status(
            tenant_id,
            shop_id,
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
            shop_id,
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
            shop_id,
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
        &state,
        &ctx,
        &req_ctx,
        "SHOP.SALES_CONFIG.UPDATE",
        "shop",
        Some(shop_id.to_string()),
        None,
        Some(serde_json::json!({
            "shop_id": shop_id,
            "config": config,
        })),
    )
    .await;

    (StatusCode::OK, Json(sales_shop_config_view(shop, config))).into_response()
}

pub(crate) async fn get_shop(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.read"]) {
        return resp.into_response();
    }

    let repo = ShopsRepo::new(state.db.pool());
    let shop = match repo.get_shop(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let tokens_repo = PlatformTokensRepo::new(state.db.pool());
    let token = match tokens_repo.get_token(tenant_id, shop_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(shop_view(shop, token.as_ref()))).into_response()
}

pub(crate) async fn create_shop(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ShopCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.write"]) {
        return resp.into_response();
    }

    let platform = req.platform.trim();
    if platform.is_empty() {
        return bad_request("platform is required").into_response();
    }
    let platform = match Platform::from_str(platform) {
        Ok(v) => v,
        Err(_) => return bad_request("bad platform").into_response(),
    };
    let platform_shop_id = req.platform_shop_id.trim();
    if platform_shop_id.is_empty() {
        return bad_request("platform_shop_id is required").into_response();
    }
    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name is required").into_response();
    }
    let status = match parse_shop_status(req.status) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };

    let repo = ShopsRepo::new(state.db.pool());
    let shop = match repo
        .create_shop(ShopCreate {
            tenant_id,
            platform,
            platform_shop_id: platform_shop_id.to_string(),
            name: name.to_string(),
            status,
            credentials: req.credentials,
            auth_meta_json: req.auth_meta_json,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let sync_repo = SyncJobsRepo::new(state.db.pool());
    let mut seeded = Vec::new();
    for job_type in ["order_sync", "refund_sync", "shipping_sync", "product_sync"] {
        match sync_repo
            .upsert_job(SyncJobUpsert {
                tenant_id,
                shop_id: shop.shop_id,
                job_type: job_type.to_string(),
                cursor: None,
                watermark: None,
                status: Some("PENDING".to_string()),
                last_error: Some(String::new()),
            })
            .await
        {
            Ok(job) => seeded.push(job),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SHOP.CREATE",
        "shop",
        Some(shop.shop_id.to_string()),
        None,
        Some(serde_json::to_value(&shop).unwrap_or(Value::Null)),
    )
    .await;

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SYNC_JOB.SEED",
        "sync_job",
        Some(shop.shop_id.to_string()),
        None,
        Some(serde_json::json!({
            "job_types": seeded.iter().map(|job| job.job_type.clone()).collect::<Vec<_>>(),
            "job_ids": seeded.iter().map(|job| job.job_id).collect::<Vec<_>>(),
        })),
    )
    .await;

    (StatusCode::OK, Json(shop_view(shop, None))).into_response()
}

pub(crate) async fn update_shop(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ShopUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.write"]) {
        return resp.into_response();
    }

    let name = req
        .name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = match parse_shop_status(req.status) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if name.is_none()
        && status.is_none()
        && req.credentials.is_none()
        && req.auth_meta_json.is_none()
    {
        return bad_request("no fields to update").into_response();
    }

    let repo = ShopsRepo::new(state.db.pool());
    let before = match repo.get_shop(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let shop = match repo
        .update_shop(
            tenant_id,
            shop_id,
            ShopUpdate {
                name,
                status,
                credentials: req.credentials,
                auth_meta_json: req.auth_meta_json,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SHOP.UPDATE",
        "shop",
        Some(shop.shop_id.to_string()),
        Some(serde_json::to_value(&before).unwrap_or(Value::Null)),
        Some(serde_json::to_value(&shop).unwrap_or(Value::Null)),
    )
    .await;

    let tokens_repo = PlatformTokensRepo::new(state.db.pool());
    let token = match tokens_repo.get_token(tenant_id, shop_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(shop_view(shop, token.as_ref()))).into_response()
}

pub(crate) async fn delete_shop(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.write"]) {
        return resp.into_response();
    }

    let repo = ShopsRepo::new(state.db.pool());
    let before = match repo.get_shop(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let shop = match repo
        .update_shop(
            tenant_id,
            shop_id,
            ShopUpdate {
                name: None,
                status: Some("DISABLED".to_string()),
                credentials: None,
                auth_meta_json: None,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SHOP.DISABLE",
        "shop",
        Some(shop.shop_id.to_string()),
        Some(serde_json::to_value(&before).unwrap_or(Value::Null)),
        Some(serde_json::to_value(&shop).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(shop_view(shop, None))).into_response()
}

pub(crate) async fn get_shop_auth_status(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.read"]) {
        return resp.into_response();
    }

    let repo = ShopsRepo::new(state.db.pool());
    let shop = match repo.get_shop(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let tokens_repo = PlatformTokensRepo::new(state.db.pool());
    let token = match tokens_repo.get_token(tenant_id, shop_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(shop_view(shop, token.as_ref()))).into_response()
}

pub(crate) async fn list_admin_shops(
    State(state): State<AppState>,
    Query(query): Query<AdminShopListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    if let Err(resp) = ensure_admin(&ctx) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.write"]) {
        return resp.into_response();
    }
    let tenant_id = match TenantId::from_str(&query.tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let platform = match query.platform.as_deref() {
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                match Platform::from_str(trimmed) {
                    Ok(v) => Some(v),
                    Err(_) => return bad_request("bad platform").into_response(),
                }
            }
        }
        None => None,
    };
    let status = match parse_shop_status(query.status) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = ShopsRepo::new(state.db.pool());
    let result = match repo
        .list_shops(
            tenant_id,
            RepoShopListFilters {
                platform,
                status,
                shop_ids: None,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let shop_ids: Vec<ShopId> = result.items.iter().map(|shop| shop.shop_id).collect();
    let tokens_repo = PlatformTokensRepo::new(state.db.pool());
    let tokens = match tokens_repo.list_tokens(tenant_id, &shop_ids).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let token_map: HashMap<ShopId, PlatformToken> = tokens
        .into_iter()
        .map(|token| (token.shop_id, token))
        .collect();

    let items = result
        .items
        .into_iter()
        .map(|shop| {
            let token = token_map.get(&shop.shop_id);
            shop_view(shop, token)
        })
        .collect();

    (
        StatusCode::OK,
        Json(ShopListResponse {
            items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn create_admin_shop(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<AdminShopCreateRequest>,
) -> impl IntoResponse {
    if let Err(resp) = ensure_admin(&ctx) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.write"]) {
        return resp.into_response();
    }

    let tenant_id = match TenantId::from_str(&req.tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let platform = req.platform.trim();
    if platform.is_empty() {
        return bad_request("platform is required").into_response();
    }
    let platform = match Platform::from_str(platform) {
        Ok(v) => v,
        Err(_) => return bad_request("bad platform").into_response(),
    };
    let platform_shop_id = req.platform_shop_id.trim();
    if platform_shop_id.is_empty() {
        return bad_request("platform_shop_id is required").into_response();
    }
    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name is required").into_response();
    }
    let status = match parse_shop_status(req.status) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };

    let repo = ShopsRepo::new(state.db.pool());
    let shop = match repo
        .create_shop(ShopCreate {
            tenant_id,
            platform,
            platform_shop_id: platform_shop_id.to_string(),
            name: name.to_string(),
            status,
            credentials: req.credentials,
            auth_meta_json: req.auth_meta_json,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let sync_repo = SyncJobsRepo::new(state.db.pool());
    let mut seeded = Vec::new();
    for job_type in ["order_sync", "refund_sync", "shipping_sync", "product_sync"] {
        match sync_repo
            .upsert_job(SyncJobUpsert {
                tenant_id,
                shop_id: shop.shop_id,
                job_type: job_type.to_string(),
                cursor: None,
                watermark: None,
                status: Some("PENDING".to_string()),
                last_error: Some(String::new()),
            })
            .await
        {
            Ok(job) => seeded.push(job),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SHOP.CREATE",
        "shop",
        Some(shop.shop_id.to_string()),
        None,
        Some(serde_json::to_value(&shop).unwrap_or(Value::Null)),
    )
    .await;

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SYNC_JOB.SEED",
        "sync_job",
        Some(shop.shop_id.to_string()),
        None,
        Some(serde_json::json!({
            "job_types": seeded.iter().map(|job| job.job_type.clone()).collect::<Vec<_>>(),
            "job_ids": seeded.iter().map(|job| job.job_id).collect::<Vec<_>>(),
        })),
    )
    .await;

    (StatusCode::OK, Json(shop_view(shop, None))).into_response()
}

pub(crate) async fn get_admin_shop(
    State(state): State<AppState>,
    Path(shop_id): Path<String>,
    Query(query): Query<AdminShopTokenQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    if let Err(resp) = ensure_admin(&ctx) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.write"]) {
        return resp.into_response();
    }
    let tenant_id = match TenantId::from_str(&query.tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };

    let repo = ShopsRepo::new(state.db.pool());
    let shop = match repo.get_shop(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let tokens_repo = PlatformTokensRepo::new(state.db.pool());
    let token = match tokens_repo.get_token(tenant_id, shop_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(shop_view(shop, token.as_ref()))).into_response()
}

pub(crate) async fn update_admin_shop(
    State(state): State<AppState>,
    Path(shop_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<AdminShopUpdateRequest>,
) -> impl IntoResponse {
    if let Err(resp) = ensure_admin(&ctx) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.write"]) {
        return resp.into_response();
    }
    let tenant_id = match TenantId::from_str(&req.tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    let name = req
        .name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = match parse_shop_status(req.status) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if name.is_none()
        && status.is_none()
        && req.credentials.is_none()
        && req.auth_meta_json.is_none()
    {
        return bad_request("no fields to update").into_response();
    }

    let repo = ShopsRepo::new(state.db.pool());
    let before = match repo.get_shop(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let shop = match repo
        .update_shop(
            tenant_id,
            shop_id,
            ShopUpdate {
                name,
                status,
                credentials: req.credentials,
                auth_meta_json: req.auth_meta_json,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SHOP.UPDATE",
        "shop",
        Some(shop.shop_id.to_string()),
        Some(serde_json::to_value(&before).unwrap_or(Value::Null)),
        Some(serde_json::to_value(&shop).unwrap_or(Value::Null)),
    )
    .await;

    let tokens_repo = PlatformTokensRepo::new(state.db.pool());
    let token = match tokens_repo.get_token(tenant_id, shop_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(shop_view(shop, token.as_ref()))).into_response()
}

pub(crate) async fn delete_admin_shop(
    State(state): State<AppState>,
    Path(shop_id): Path<String>,
    Query(query): Query<AdminShopTokenQuery>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    if let Err(resp) = ensure_admin(&ctx) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.write"]) {
        return resp.into_response();
    }
    let tenant_id = match TenantId::from_str(&query.tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };

    let repo = ShopsRepo::new(state.db.pool());
    let before = match repo.get_shop(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let shop = match repo
        .update_shop(
            tenant_id,
            shop_id,
            ShopUpdate {
                name: None,
                status: Some("DISABLED".to_string()),
                credentials: None,
                auth_meta_json: None,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SHOP.DISABLE",
        "shop",
        Some(shop.shop_id.to_string()),
        Some(serde_json::to_value(&before).unwrap_or(Value::Null)),
        Some(serde_json::to_value(&shop).unwrap_or(Value::Null)),
    )
    .await;

    let tokens_repo = PlatformTokensRepo::new(state.db.pool());
    let token = match tokens_repo.get_token(tenant_id, shop_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(shop_view(shop, token.as_ref()))).into_response()
}

pub(crate) async fn get_admin_shop_token(
    State(state): State<AppState>,
    Path(shop_id): Path<String>,
    Query(query): Query<AdminShopTokenQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    if let Err(resp) = ensure_admin(&ctx) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.write"]) {
        return resp.into_response();
    }
    let tenant_id = match TenantId::from_str(&query.tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };

    let tokens_repo = PlatformTokensRepo::new(state.db.pool());
    let token = match tokens_repo.get_token(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("token not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(token)).into_response()
}

pub(crate) async fn list_admin_sync_jobs(
    State(state): State<AppState>,
    Query(query): Query<AdminSyncJobListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    if let Err(resp) = ensure_admin(&ctx) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.write"]) {
        return resp.into_response();
    }
    let tenant_id = match TenantId::from_str(&query.tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match query.shop_id.as_deref() {
        Some(raw) => match ShopId::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    let job_type = query
        .job_type
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = query
        .status
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = SyncJobsRepo::new(state.db.pool());
    let result = match repo
        .list_jobs(
            tenant_id,
            RepoSyncJobListFilters {
                shop_id,
                shop_ids: None,
                job_type,
                status,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(SyncJobListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn start_shop_authorize(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ShopAuthorizeRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.write"]) {
        return resp.into_response();
    }

    let repo = ShopsRepo::new(state.db.pool());
    let shop = match repo.get_shop(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if shop.status == "DISABLED" {
        return conflict("shop disabled").into_response();
    }

    let auth_ctx = match repo.get_shop_auth(shop_id).await {
        Ok(v) => v,
        Err(e) => {
            let message = e.to_string();
            if message.contains("no rows") {
                return conflict("shop not authorized").into_response();
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, message).into_response();
        }
    };
    let redirect_uri = req
        .redirect_uri
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let state_token = uuid::Uuid::new_v4().to_string();
    let adapters = AdapterRegistry::new();
    let adapter = adapters.platform_adapter(shop.platform);
    let Some(authorize_url) = adapter.build_authorize_url(&auth_ctx, redirect_uri, &state_token)
    else {
        return bad_request("authorize not supported").into_response();
    };

    let auth_meta = merge_auth_meta_json(
        shop.auth_meta_json,
        None,
        serde_json::json!({
            "oauth_status": "PENDING",
            "oauth_state": state_token,
            "oauth_error": serde_json::Value::Null,
            "oauth_error_description": serde_json::Value::Null,
            "oauth_updated_at": Utc::now().to_rfc3339(),
        }),
    );
    let _ = repo
        .update_shop(
            tenant_id,
            shop_id,
            ShopUpdate {
                name: None,
                status: None,
                credentials: None,
                auth_meta_json: Some(auth_meta),
            },
        )
        .await;

    (
        StatusCode::OK,
        Json(ShopAuthorizeResponse {
            authorize_url,
            state: state_token,
        }),
    )
        .into_response()
}

pub(crate) async fn shop_auth_callback(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ShopAuthCallbackRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.write"]) {
        return resp.into_response();
    }

    let mut access_token = req.access_token.and_then(non_empty);
    let mut refresh_token = req.refresh_token.and_then(non_empty);
    let authorization_code = req.code.and_then(non_empty);
    let redirect_uri = req.redirect_uri.and_then(non_empty);
    let state_value = req.state.and_then(non_empty);
    let mut auth_error = req.error.and_then(non_empty);
    let mut auth_error_description = req.error_description.and_then(non_empty);
    let auth_meta_override = req.auth_meta_json.clone();
    let mut expires_at = match req.expires_at.as_deref() {
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                match DateTime::parse_from_rfc3339(trimmed) {
                    Ok(v) => Some(v.with_timezone(&Utc)),
                    Err(_) => return bad_request("bad expires_at").into_response(),
                }
            }
        }
        None => None,
    };
    let mut scopes = req.scopes.and_then(non_empty);

    let repo = ShopsRepo::new(state.db.pool());
    let before_shop = match repo.get_shop(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let tokens_repo = PlatformTokensRepo::new(state.db.pool());
    let before_token = match tokens_repo.get_token(tenant_id, shop_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if auth_error.is_none() {
        if let (Some(expected), Some(provided)) = (
            before_shop
                .auth_meta_json
                .get("oauth_state")
                .and_then(|v| v.as_str()),
            state_value.as_deref(),
        ) {
            if expected != provided {
                auth_error = Some("state_mismatch".to_string());
                auth_error_description = Some(format!("expected {expected}, got {provided}"));
            }
        }
    }
    if auth_error.is_none() && access_token.is_none() && refresh_token.is_none() {
        if let Some(code) = authorization_code.as_deref() {
            let shop_auth = match repo.get_shop_auth(shop_id).await {
                Ok(v) => v,
                Err(_) => return forbidden("shop auth required").into_response(),
            };
            let adapters = AdapterRegistry::new();
            let adapter = adapters.platform_adapter(shop_auth.platform);
            match adapter
                .exchange_authorization_code(&shop_auth, code, redirect_uri.clone())
                .await
            {
                Ok(result) => {
                    access_token = result.access_token;
                    refresh_token = result.refresh_token;
                    if expires_at.is_none() {
                        expires_at = result.expires_at;
                    }
                    if scopes.is_none() {
                        scopes = result.scopes;
                    }
                }
                Err(err) => {
                    auth_error = Some("oauth_exchange_failed".to_string());
                    auth_error_description = Some(err.to_string());
                }
            }
        }
    }

    if let Some(error) = auth_error {
        let auth_meta = merge_auth_meta_json(
            before_shop.auth_meta_json.clone(),
            auth_meta_override.clone(),
            serde_json::json!({
                "oauth_status": "FAILED",
                "oauth_state": state_value,
                "oauth_error": error,
                "oauth_error_description": auth_error_description,
                "oauth_updated_at": Utc::now().to_rfc3339(),
            }),
        );
        let shop = match repo
            .update_shop(
                tenant_id,
                shop_id,
                ShopUpdate {
                    name: None,
                    status: Some("EXPIRED".to_string()),
                    credentials: None,
                    auth_meta_json: Some(auth_meta),
                },
            )
            .await
        {
            Ok(Some(v)) => v,
            Ok(None) => return not_found("shop not found").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };

        record_audit_log(
            &state,
            &ctx,
            &req_ctx,
            "SHOP.AUTH_CALLBACK",
            "shop",
            Some(shop.shop_id.to_string()),
            Some(serde_json::to_value(&before_shop).unwrap_or(Value::Null)),
            Some(serde_json::to_value(&shop).unwrap_or(Value::Null)),
        )
        .await;

        return (StatusCode::OK, Json(shop_view(shop, before_token.as_ref()))).into_response();
    }

    if access_token.is_none() && refresh_token.is_none() {
        return bad_request("missing token").into_response();
    }
    let token = match tokens_repo
        .upsert_token(PlatformTokenUpsert {
            tenant_id,
            shop_id,
            token_status: None,
            access_token,
            refresh_token,
            expires_at,
            scopes,
            last_refreshed_at: Some(Utc::now()),
            refresh_fail_count: Some(0),
            last_error: None,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let status = if expires_at.map(|ts| ts <= Utc::now()).unwrap_or(false) {
        "EXPIRED"
    } else {
        "AUTHORIZED"
    };
    let auth_meta = merge_auth_meta_json(
        before_shop.auth_meta_json.clone(),
        auth_meta_override,
        serde_json::json!({
            "oauth_status": "SUCCESS",
            "oauth_state": state_value,
            "oauth_error": serde_json::Value::Null,
            "oauth_error_description": serde_json::Value::Null,
            "oauth_updated_at": Utc::now().to_rfc3339(),
        }),
    );
    let shop = match repo
        .update_shop(
            tenant_id,
            shop_id,
            ShopUpdate {
                name: None,
                status: Some(status.to_string()),
                credentials: None,
                auth_meta_json: Some(auth_meta),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SHOP.AUTH_CALLBACK",
        "shop",
        Some(shop.shop_id.to_string()),
        Some(serde_json::to_value(&before_shop).unwrap_or(Value::Null)),
        Some(serde_json::to_value(&shop).unwrap_or(Value::Null)),
    )
    .await;

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "TOKEN.UPSERT",
        "platform_token",
        Some(shop.shop_id.to_string()),
        before_token.as_ref().map(token_audit_payload),
        Some(token_audit_payload(&token)),
    )
    .await;

    (StatusCode::OK, Json(shop_view(shop, Some(&token)))).into_response()
}

pub(crate) async fn request_token_refresh(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.write"]) {
        return resp.into_response();
    }

    let repo = ShopsRepo::new(state.db.pool());
    let shop = match repo.get_shop(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let payload = TokenRefreshRequestedPayload {
        tenant_id,
        shop_id,
        platform: shop.platform,
    };
    let outbox_repo = OutboxRepo::new(state.db.pool());
    let outbox_id = match outbox_repo
        .enqueue_with_trace(
            tenant_id,
            "TOKEN_REFRESH_REQUESTED",
            &serde_json::to_value(&payload).unwrap_or(Value::Null),
            Some(req_ctx.trace_id.as_str()),
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "TOKEN.REFRESH_REQUEST",
        "platform_token",
        Some(shop.shop_id.to_string()),
        None,
        Some(serde_json::json!({
            "outbox_id": outbox_id,
            "platform": shop.platform.to_string(),
        })),
    )
    .await;

    let tokens_repo = PlatformTokensRepo::new(state.db.pool());
    let token = match tokens_repo.get_token(tenant_id, shop_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let (token_status, expires_at, last_error) = match token {
        Some(token) => (
            Some(token.token_status),
            token.expires_at,
            Some(token.last_error),
        ),
        None => (None, None, None),
    };

    (
        StatusCode::OK,
        Json(TokenRefreshRequestResponse {
            outbox_id,
            token_status,
            expires_at,
            last_error,
        }),
    )
        .into_response()
}

pub(crate) async fn list_sync_jobs(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<SyncJobListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.write"]) {
        return resp.into_response();
    }

    let shop_id = match query.shop_id.as_deref() {
        Some(raw) => match ShopId::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    if let Some(shop_id) = shop_id {
        if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
            return resp.into_response();
        }
    }
    let shop_ids = shop_scope_filter(&ctx);
    if let (Some(shop_id), Some(allowed)) = (shop_id, &shop_ids) {
        if !allowed.contains(&shop_id) {
            return forbidden("shop access denied").into_response();
        }
    }

    let job_type = query
        .job_type
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = query
        .status
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = SyncJobsRepo::new(state.db.pool());
    let result = match repo
        .list_jobs(
            tenant_id,
            RepoSyncJobListFilters {
                shop_id,
                shop_ids,
                job_type,
                status,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(SyncJobListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn list_sync_job_runs(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<SyncJobRunListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.write"]) {
        return resp.into_response();
    }

    let job_id = match query.job_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad job_id").into_response(),
        },
        None => None,
    };
    let shop_id = match query.shop_id.as_deref() {
        Some(raw) => match ShopId::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    if let Some(shop_id) = shop_id {
        if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
            return resp.into_response();
        }
    }
    let shop_ids = shop_scope_filter(&ctx);
    if let (Some(shop_id), Some(allowed)) = (shop_id, &shop_ids) {
        if !allowed.contains(&shop_id) {
            return forbidden("shop access denied").into_response();
        }
    }

    let parse_dt = |raw: &str, label: &str| -> Result<DateTime<Utc>, Response> {
        DateTime::parse_from_rfc3339(raw)
            .map(|value| value.with_timezone(&Utc))
            .map_err(|_| bad_request(&format!("bad {label}")).into_response())
    };
    let started_from = match query.started_from.as_deref() {
        Some(raw) => match parse_dt(raw, "started_from") {
            Ok(v) => Some(v),
            Err(resp) => return resp,
        },
        None => None,
    };
    let started_to = match query.started_to.as_deref() {
        Some(raw) => match parse_dt(raw, "started_to") {
            Ok(v) => Some(v),
            Err(resp) => return resp,
        },
        None => None,
    };

    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let repo = SyncJobRunsRepo::new(state.db.pool());
    let result = match repo
        .list_runs(
            tenant_id,
            SyncJobRunListFilters {
                job_id,
                shop_id,
                shop_ids,
                job_type: query.job_type.and_then(non_empty),
                status: query.status.and_then(non_empty),
                started_from,
                started_to,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(SyncJobRunListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn request_sync_job_run_internal(
    state: &AppState,
    tenant_id: TenantId,
    shop_id: ShopId,
    job_type: &str,
    cursor: Option<String>,
    watermark: Option<DateTime<Utc>>,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    audit_action: &str,
    audit_extra: Value,
    enforce_sales_config: bool,
) -> Result<SyncJob, Response> {
    let job_type = job_type.trim();
    if job_type.is_empty() {
        return Err(bad_request("job_type is required").into_response());
    }
    if !["order_sync", "refund_sync", "shipping_sync", "product_sync"].contains(&job_type) {
        return Err(bad_request("invalid job_type").into_response());
    }

    if enforce_sales_config {
        let sales_config = load_sales_config_for_shop(state.db.pool(), tenant_id, shop_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response())?;
        if job_type == "order_sync" && !sales_config.auto_download_orders {
            return Err(forbidden("sales auto download disabled").into_response());
        }
        if job_type == "refund_sync" && !sales_config.enable_after_sales_download {
            return Err(forbidden("after-sales download disabled").into_response());
        }
        if job_type == "shipping_sync" && !sales_config.enable_logistics_upload {
            return Err(forbidden("logistics upload disabled").into_response());
        }
    }

    let sync_flag = match job_type {
        "order_sync" => Some(FeatureFlagKey::OrderSync),
        "refund_sync" => Some(FeatureFlagKey::AfterSalesSync),
        "shipping_sync" => Some(FeatureFlagKey::ShippingWrite),
        "product_sync" => None,
        _ => Some(FeatureFlagKey::OrderSync),
    };
    if let Some(sync_flag) = sync_flag {
        let sync_enabled = feature_flag_enabled(state, tenant_id, Some(shop_id), sync_flag)
            .await
            .map_err(|resp| resp.into_response())?;
        if !sync_enabled {
            return Err(feature_flag_forbidden(sync_flag).into_response());
        }
    }

    let cursor = cursor
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let repo = SyncJobsRepo::new(state.db.pool());
    let job = repo
        .upsert_job(SyncJobUpsert {
            tenant_id,
            shop_id,
            job_type: job_type.to_string(),
            cursor,
            watermark,
            status: Some("PENDING".to_string()),
            last_error: Some("FORCE_RUN".to_string()),
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response())?;

    let runs_repo = SyncJobRunsRepo::new(state.db.pool());
    let now = Utc::now();
    runs_repo
        .insert_run(SyncJobRunCreate {
            tenant_id,
            job_id: job.job_id,
            shop_id: job.shop_id,
            job_type: job.job_type.clone(),
            status: "REQUESTED".to_string(),
            item_count: 0,
            duration_ms: 0,
            error: String::new(),
            started_at: now,
            finished_at: now,
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response())?;

    record_audit_log(
        state,
        ctx,
        req_ctx,
        audit_action,
        "sync_job",
        Some(job.job_id.to_string()),
        None,
        Some(serde_json::json!({
            "shop_id": job.shop_id,
            "job_type": job.job_type,
            "criteria": audit_extra,
        })),
    )
    .await;

    Ok(job)
}

pub(crate) async fn run_sync_job(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<SyncJobRunRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["settings.read", "channels.write"]) {
        return resp.into_response();
    }

    let watermark = match parse_datetime_optional(req.watermark.as_deref(), "watermark") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    match request_sync_job_run_internal(
        &state,
        tenant_id,
        shop_id,
        &req.job_type,
        req.cursor,
        watermark,
        &ctx,
        &req_ctx,
        "SYNC_JOB.RUN",
        serde_json::json!({}),
        true,
    )
    .await
    {
        Ok(job) => (StatusCode::OK, Json(job)).into_response(),
        Err(resp) => resp,
    }
}

pub(crate) async fn download_platform_orders_manual(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<PlatformOrderManualDownloadRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.download") {
        return resp.into_response();
    }

    let watermark = match parse_datetime_optional(
        req.watermark.as_deref().or(req.last_time.as_deref()),
        "watermark",
    ) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };

    let criteria = serde_json::json!({
        "mode": "MANUAL",
        "cursor": req.cursor,
        "watermark": req.watermark.or(req.last_time),
    });
    match request_sync_job_run_internal(
        &state,
        tenant_id,
        shop_id,
        "order_sync",
        req.cursor,
        watermark,
        &ctx,
        &req_ctx,
        "PLATFORM_ORDER.DOWNLOAD_MANUAL",
        criteria.clone(),
        false,
    )
    .await
    {
        Ok(job) => (
            StatusCode::OK,
            Json(PlatformOrderDownloadResponse { job, criteria }),
        )
            .into_response(),
        Err(resp) => resp,
    }
}

pub(crate) async fn download_platform_orders_by_time_range(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<PlatformOrderTimeRangeDownloadRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.download") {
        return resp.into_response();
    }

    let start_time = match parse_datetime_optional(Some(req.start_time.as_str()), "start_time") {
        Ok(Some(v)) => v,
        Ok(None) => return bad_request("start_time is required").into_response(),
        Err(resp) => return resp.into_response(),
    };
    let end_time = match parse_datetime_optional(Some(req.end_time.as_str()), "end_time") {
        Ok(Some(v)) => v,
        Ok(None) => return bad_request("end_time is required").into_response(),
        Err(resp) => return resp.into_response(),
    };
    if start_time > end_time {
        return bad_request("start_time must be <= end_time").into_response();
    }

    let criteria = serde_json::json!({
        "mode": "TIME_RANGE",
        "start_time": start_time.to_rfc3339(),
        "end_time": end_time.to_rfc3339(),
        "cursor": req.cursor,
    });
    match request_sync_job_run_internal(
        &state,
        tenant_id,
        shop_id,
        "order_sync",
        req.cursor,
        Some(start_time),
        &ctx,
        &req_ctx,
        "PLATFORM_ORDER.DOWNLOAD_TIME_RANGE",
        criteria.clone(),
        false,
    )
    .await
    {
        Ok(job) => (
            StatusCode::OK,
            Json(PlatformOrderDownloadResponse { job, criteria }),
        )
            .into_response(),
        Err(resp) => resp,
    }
}

pub(crate) async fn download_platform_orders_by_order_nos(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<PlatformOrderNoDownloadRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.download") {
        return resp.into_response();
    }

    let order_nos = normalize_string_list(req.order_nos);
    if order_nos.is_empty() {
        return bad_request("order_nos required").into_response();
    }
    if order_nos.len() > 200 {
        return bad_request("order_nos exceeds 200").into_response();
    }

    let shops_repo = ShopsRepo::new(state.db.pool());
    let shop = match shops_repo.get_shop(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let shop_auth = match shops_repo.get_shop_auth(shop_id).await {
        Ok(v) => v,
        Err(_) => return forbidden("shop auth required").into_response(),
    };
    if shop_auth.tenant_id != tenant_id {
        return forbidden("shop access denied").into_response();
    }
    let pdd_only = match pdd_only_mode_enabled(&state, tenant_id).await {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if pdd_only && shop.platform != Platform::Pdd {
        return pdd_only_forbidden().into_response();
    }
    let order_sync_enabled =
        match feature_flag_enabled(&state, tenant_id, Some(shop_id), FeatureFlagKey::OrderSync)
            .await
        {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !order_sync_enabled {
        return feature_flag_forbidden(FeatureFlagKey::OrderSync).into_response();
    }

    let adapters = AdapterRegistry::new();
    let adapter = adapters.platform_adapter(shop.platform);
    let orders_repo = OrdersRepo::new(state.db.pool());
    let outbox_repo = OutboxRepo::new(state.db.pool());
    let mut items = Vec::with_capacity(order_nos.len());
    let mut success_count = 0_i64;
    let mut failure_count = 0_i64;

    for order_no in order_nos {
        match adapter.fetch_order(&shop_auth, &order_no).await {
            Ok(mut projection) => {
                projection.tenant_id = tenant_id;
                projection.shop_id = shop_id;
                projection.platform = shop.platform;
                if let Err(err) =
                    apply_order_item_sku_mappings(state.db.pool(), &outbox_repo, &mut projection)
                        .await
                {
                    failure_count += 1;
                    items.push(PlatformOrderNoDownloadResult {
                        platform_order_id: order_no,
                        success: false,
                        order_id: None,
                        message: Some(format!("sku mapping apply failed: {err}")),
                    });
                    continue;
                }
                match orders_repo.upsert_order_projection(&projection).await {
                    Ok(order_id) => {
                        let payload = serde_json::json!(OrderUpsertedPayload {
                            tenant_id,
                            platform: shop.platform,
                            shop_id,
                            platform_order_id: projection.platform_order_id.clone(),
                            order_id,
                        });
                        let event_error = outbox_repo
                            .enqueue(tenant_id, "ORDER_IMPORTED", &payload)
                            .await
                            .err()
                            .map(|e| e.to_string());
                        success_count += 1;
                        items.push(PlatformOrderNoDownloadResult {
                            platform_order_id: projection.platform_order_id,
                            success: true,
                            order_id: Some(order_id),
                            message: event_error
                                .map(|v| format!("order imported, event enqueue failed: {v}")),
                        });
                    }
                    Err(err) => {
                        failure_count += 1;
                        items.push(PlatformOrderNoDownloadResult {
                            platform_order_id: order_no,
                            success: false,
                            order_id: None,
                            message: Some(format!("order upsert failed: {err}")),
                        });
                    }
                }
            }
            Err(err) => {
                failure_count += 1;
                items.push(PlatformOrderNoDownloadResult {
                    platform_order_id: order_no,
                    success: false,
                    order_id: None,
                    message: Some(err.to_string()),
                });
            }
        }
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "PLATFORM_ORDER.DOWNLOAD_BY_ORDER_NOS",
        "shop",
        Some(shop_id.to_string()),
        None,
        Some(serde_json::json!({
            "shop_id": shop_id,
            "total": items.len(),
            "success_count": success_count,
            "failure_count": failure_count,
            "items": &items,
        })),
    )
    .await;

    (
        StatusCode::OK,
        Json(PlatformOrderNoDownloadResponse {
            total: success_count + failure_count,
            success_count,
            failure_count,
            items,
        }),
    )
        .into_response()
}

pub(crate) async fn get_order_sync_settings(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) =
        ensure_permission_any(&ctx, &["settings.read", "channels.read", "orders.read"])
    {
        return resp.into_response();
    }

    let config = match load_sales_config_for_shop(state.db.pool(), tenant_id, shop_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(OrderSyncSettingsView {
            shop_id,
            auto_download_orders: config.auto_download_orders,
            download_interval_minutes: config.download_interval_minutes,
        }),
    )
        .into_response()
}

pub(crate) async fn update_order_sync_settings(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OrderSyncSettingsUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) =
        ensure_permission_any(&ctx, &["settings.read", "channels.write", "orders.write"])
    {
        return resp.into_response();
    }

    if let Some(interval) = req.download_interval_minutes {
        if interval <= 0 {
            return bad_request("download_interval_minutes must be positive").into_response();
        }
        let allowed_intervals = [5_i64, 10, 15, 30, 60];
        if !allowed_intervals.contains(&interval) {
            return bad_request("download_interval_minutes must be 5/10/15/30/60").into_response();
        }
    }

    let shops_repo = ShopsRepo::new(state.db.pool());
    let shop = match shops_repo.get_shop(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let mut config = match load_sales_config_for_shop(state.db.pool(), tenant_id, shop_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Some(v) = req.auto_download_orders {
        config.auto_download_orders = v;
    }
    if let Some(v) = req.download_interval_minutes {
        config.download_interval_minutes = v;
    }

    let saved = match save_sales_shop_config(
        &state,
        tenant_id,
        shop,
        config,
        &ctx,
        &req_ctx,
        "SHOP.ORDER_SYNC_SETTINGS.UPDATE",
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    (
        StatusCode::OK,
        Json(OrderSyncSettingsView {
            shop_id: saved.shop_id,
            auto_download_orders: saved.auto_download_orders,
            download_interval_minutes: saved.download_interval_minutes,
        }),
    )
        .into_response()
}

pub(crate) async fn get_inventory_sync_settings(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) =
        ensure_permission_any(&ctx, &["settings.read", "channels.read", "inventory.read"])
    {
        return resp.into_response();
    }

    let config = match load_sales_config_for_shop(state.db.pool(), tenant_id, shop_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(InventorySyncSettingsView {
            shop_id,
            enable_stock_sync: config.enable_stock_sync,
        }),
    )
        .into_response()
}

pub(crate) async fn update_inventory_sync_settings(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<InventorySyncSettingsUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(
        &ctx,
        &["settings.read", "channels.write", "inventory.write"],
    ) {
        return resp.into_response();
    }

    let shops_repo = ShopsRepo::new(state.db.pool());
    let shop = match shops_repo.get_shop(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let mut config = match load_sales_config_for_shop(state.db.pool(), tenant_id, shop_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    config.enable_stock_sync = req.enable_stock_sync;

    let saved = match save_sales_shop_config(
        &state,
        tenant_id,
        shop,
        config,
        &ctx,
        &req_ctx,
        "SHOP.INVENTORY_SYNC_SETTINGS.UPDATE",
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    (
        StatusCode::OK,
        Json(InventorySyncSettingsView {
            shop_id: saved.shop_id,
            enable_stock_sync: saved.enable_stock_sync,
        }),
    )
        .into_response()
}

pub(crate) async fn get_after_sales_sync_settings(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) =
        ensure_permission_any(&ctx, &["settings.read", "channels.read", "aftersales.read"])
    {
        return resp.into_response();
    }

    let config = match load_sales_config_for_shop(state.db.pool(), tenant_id, shop_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(AfterSalesSyncSettingsView {
            shop_id,
            enable_after_sales_download: config.enable_after_sales_download,
        }),
    )
        .into_response()
}

pub(crate) async fn update_after_sales_sync_settings(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<AfterSalesSyncSettingsUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(
        &ctx,
        &["settings.read", "channels.write", "aftersales.write"],
    ) {
        return resp.into_response();
    }

    let shops_repo = ShopsRepo::new(state.db.pool());
    let shop = match shops_repo.get_shop(tenant_id, shop_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shop not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let mut config = match load_sales_config_for_shop(state.db.pool(), tenant_id, shop_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    config.enable_after_sales_download = req.enable_after_sales_download;

    let saved = match save_sales_shop_config(
        &state,
        tenant_id,
        shop,
        config,
        &ctx,
        &req_ctx,
        "SHOP.AFTER_SALES_SYNC_SETTINGS.UPDATE",
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    (
        StatusCode::OK,
        Json(AfterSalesSyncSettingsView {
            shop_id: saved.shop_id,
            enable_after_sales_download: saved.enable_after_sales_download,
        }),
    )
        .into_response()
}

pub(crate) async fn run_platform_inventory_sync(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<PlatformInventorySyncRunRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    let sync_enabled = match feature_flag_enabled(
        &state,
        tenant_id,
        Some(shop_id),
        FeatureFlagKey::InventorySync,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if !sync_enabled {
        return feature_flag_forbidden(FeatureFlagKey::InventorySync).into_response();
    }

    let sales_config = match load_sales_config_for_shop(state.db.pool(), tenant_id, shop_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if !sales_config.enable_stock_sync {
        return forbidden("stock sync disabled by sales config").into_response();
    }

    let sku_id = match req.sku_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };
    let warehouse_id = match req.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let limit = req.limit.unwrap_or(50).clamp(1, 200);

    let repo = StockSyncJobsRepo::new(state.db.pool());
    let list = match repo
        .list_jobs(
            tenant_id,
            StockSyncJobListFilters {
                shop_id: Some(shop_id),
                sku_id,
                warehouse_id,
                status: req.status,
                offset: 0,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let mut requested = Vec::new();
    let mut skipped = 0_i64;
    for job in list.items {
        match repo.request_job_run(tenant_id, job.job_id).await {
            Ok(Some(updated)) => requested.push(stock_sync_job_response(updated)),
            Ok(None) => skipped += 1,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "PLATFORM_INVENTORY.SYNC_RUN",
        "shop",
        Some(shop_id.to_string()),
        None,
        Some(serde_json::json!({
            "shop_id": shop_id,
            "requested": requested.len(),
            "skipped": skipped,
        })),
    )
    .await;

    (
        StatusCode::OK,
        Json(PlatformInventorySyncRunResponse {
            requested: requested.len() as i64,
            skipped,
            jobs: requested,
        }),
    )
        .into_response()
}

pub(crate) async fn run_platform_after_sales_sync(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<PlatformAfterSalesSyncRunRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let watermark = match parse_datetime_optional(
        req.watermark.as_deref().or(req.last_time.as_deref()),
        "watermark",
    ) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    match request_sync_job_run_internal(
        &state,
        tenant_id,
        shop_id,
        "refund_sync",
        req.cursor,
        watermark,
        &ctx,
        &req_ctx,
        "PLATFORM_AFTER_SALES.SYNC_RUN",
        serde_json::json!({ "mode": "AFTER_SALES" }),
        false,
    )
    .await
    {
        Ok(job) => (StatusCode::OK, Json(job)).into_response(),
        Err(resp) => resp,
    }
}

pub(crate) async fn run_platform_return_exchange_sync(
    State(state): State<AppState>,
    Path((tenant_id, shop_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<PlatformAfterSalesSyncRunRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.write") {
        return resp.into_response();
    }

    let watermark = match parse_datetime_optional(
        req.watermark.as_deref().or(req.last_time.as_deref()),
        "watermark",
    ) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    match request_sync_job_run_internal(
        &state,
        tenant_id,
        shop_id,
        "refund_sync",
        req.cursor,
        watermark,
        &ctx,
        &req_ctx,
        "PLATFORM_RETURN_EXCHANGE.SYNC_RUN",
        serde_json::json!({ "mode": "RETURN_EXCHANGE" }),
        false,
    )
    .await
    {
        Ok(job) => (StatusCode::OK, Json(job)).into_response(),
        Err(resp) => resp,
    }
}
