use super::*;

pub(crate) async fn list_channel_sku_mappings(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ChannelSkuMappingListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "channel_sku_mappings.read") {
        return resp.into_response();
    }

    let shop_id = match query.shop_id.as_deref() {
        Some(v) => match ShopId::from_str(v) {
            Ok(s) => Some(s),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    let platform = match query.platform.as_deref() {
        Some(v) => match Platform::from_str(v) {
            Ok(p) => Some(p),
            Err(_) => return bad_request("bad platform").into_response(),
        },
        None => None,
    };
    let sku_id = match query.sku_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };
    let cursor = match query.cursor.as_deref() {
        Some(raw) => match ChannelSkuMappingCursor::decode(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad cursor").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let result = match state
        .channel_sku_mappings_repo
        .list_mappings(
            tenant_id,
            ChannelSkuMappingListFilters {
                shop_id,
                platform,
                sku_id,
                platform_sku_id: query.platform_sku_id,
                platform_outer_sku_id: query.platform_outer_sku_id,
                status: query.status,
                cursor,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ListResponse {
        data: result.items,
        next_cursor: result.next_cursor.map(|c| c.encode()),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) fn mapping_view_from_repo(view: ChannelSkuMappingView) -> MappingView {
    MappingView {
        mapping_id: view.mapping_id,
        tenant_id: view.tenant_id,
        platform: view.platform,
        shop_id: view.shop_id,
        platform_sku_id: view.platform_sku_id,
        outer_sku_id: view.platform_outer_sku_id,
        sku_id: view.sku_id,
        sku_code: view.sku_code,
        barcode: view.barcode,
        status: view.status,
        conflict_reason: view.conflict_reason,
        match_source: Some(view.match_source),
        created_at: view.created_at,
        updated_at: view.updated_at,
    }
}

pub(crate) async fn mapping_view_from_mapping(
    pool: &PgPool,
    mapping: ChannelSkuMapping,
) -> anyhow::Result<MappingView> {
    let sku_repo = SkusRepo::new(pool);
    let sku = sku_repo
        .get_sku(mapping.tenant_id, mapping.sku_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("sku not found for mapping"))?;

    Ok(MappingView {
        mapping_id: mapping.mapping_id,
        tenant_id: mapping.tenant_id,
        platform: mapping.platform,
        shop_id: mapping.shop_id,
        platform_sku_id: mapping.platform_sku_id,
        outer_sku_id: mapping.platform_outer_sku_id,
        sku_id: mapping.sku_id,
        sku_code: sku.sku_code,
        barcode: sku.barcode,
        status: mapping.status,
        conflict_reason: mapping.conflict_reason,
        match_source: Some(mapping.match_source),
        created_at: mapping.created_at,
        updated_at: mapping.updated_at,
    })
}

pub(crate) async fn build_mapping_conflict(
    pool: &PgPool,
    existing: &ChannelSkuMapping,
    incoming_sku_id: &str,
) -> anyhow::Result<MappingConflict> {
    let sku_repo = SkusRepo::new(pool);
    let sku = sku_repo
        .get_sku(existing.tenant_id, existing.sku_id)
        .await?;
    Ok(MappingConflict {
        platform: existing.platform.to_string(),
        shop_id: existing.shop_id.to_string(),
        platform_sku_id: existing.platform_sku_id.clone(),
        incoming_sku_id: incoming_sku_id.to_string(),
        existing_mapping_id: existing.mapping_id.to_string(),
        existing_sku_id: existing.sku_id.to_string(),
        existing_sku_code: sku.as_ref().map(|s| s.sku_code.clone()),
        existing_barcode: sku.and_then(|s| s.barcode),
        conflict_reason: "SKU_MISMATCH".to_string(),
    })
}

pub(crate) fn mapping_conflict_response(
    conflict: MappingConflict,
) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::CONFLICT,
        Json(ErrorResponse {
            code: "mapping_conflict".to_string(),
            message: "平台 SKU 已存在映射".to_string(),
            details: Some(serde_json::to_value(conflict).unwrap_or(Value::Null)),
            request_id: None,
        }),
    )
}

pub(crate) async fn list_mappings(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<MappingListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "channel_sku_mappings.read") {
        return resp.into_response();
    }

    let shop_id = match query.shop_id.as_deref() {
        Some(v) => match ShopId::from_str(v) {
            Ok(s) => Some(s),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    let platform = match query.platform.as_deref() {
        Some(v) => match Platform::from_str(v) {
            Ok(p) => Some(p),
            Err(_) => return bad_request("bad platform").into_response(),
        },
        None => None,
    };
    let platform_sku_id = query
        .platform_sku_id
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string());
    let sku_code = query
        .sku_code
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string());
    let barcode = query
        .barcode
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string());
    let status = query
        .status
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string());
    let conflict_reason = query
        .conflict_reason
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string());
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let result = match ChannelSkuMappingsRepo::new(state.db.pool())
        .list_mapping_views(
            tenant_id,
            ChannelSkuMappingViewListFilters {
                shop_id,
                platform,
                platform_sku_id,
                sku_code,
                barcode,
                status,
                conflict_reason,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items = result
        .items
        .into_iter()
        .map(mapping_view_from_repo)
        .collect();

    (
        StatusCode::OK,
        Json(MappingListResponse {
            items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn create_mapping(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<MappingPayload>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "channel_sku_mappings.write") {
        return resp.into_response();
    }

    let shop_id = match ShopId::from_str(&req.shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    let platform = match Platform::from_str(&req.platform) {
        Ok(v) => v,
        Err(_) => return bad_request("bad platform").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(&req.sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };
    let platform_sku_id = req.platform_sku_id.trim();
    if platform_sku_id.is_empty() {
        return bad_request("platform_sku_id required").into_response();
    }
    let resolve_conflict = req.resolve_conflict.unwrap_or(false);

    let repo = ChannelSkuMappingsRepo::new(state.db.pool());
    if let Ok(Some(existing)) = repo
        .get_mapping_by_platform_sku(tenant_id, shop_id, platform, platform_sku_id)
        .await
    {
        if existing.sku_id != sku_id {
            let conflict = match build_mapping_conflict(state.db.pool(), &existing, &req.sku_id)
                .await
            {
                Ok(conflict) => conflict,
                Err(err) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response()
                }
            };
            if resolve_conflict {
                let mapping = match repo
                    .update_mapping(
                        tenant_id,
                        existing.mapping_id,
                        ChannelSkuMappingUpdate {
                            platform_sku_id: None,
                            platform_outer_sku_id: req.outer_sku_id.clone(),
                            sku_id: Some(sku_id),
                            status: Some("ACTIVE".to_string()),
                            conflict_reason: Some(String::new()),
                            confidence: None,
                            match_source: Some("MANUAL".to_string()),
                        },
                    )
                    .await
                {
                    Ok(Some(updated)) => updated,
                    Ok(None) => existing,
                    Err(e) => {
                        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
                    }
                };
                let view = match mapping_view_from_mapping(state.db.pool(), mapping).await {
                    Ok(v) => v,
                    Err(e) => {
                        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
                    }
                };
                return (StatusCode::OK, Json(view)).into_response();
            }

            let _ = repo
                .update_mapping(
                    tenant_id,
                    existing.mapping_id,
                    ChannelSkuMappingUpdate {
                        platform_sku_id: None,
                        platform_outer_sku_id: None,
                        sku_id: None,
                        status: Some("CONFLICT".to_string()),
                        conflict_reason: Some(conflict.conflict_reason.clone()),
                        confidence: None,
                        match_source: None,
                    },
                )
                .await;

            return mapping_conflict_response(conflict).into_response();
        }

        let mapping = if req.outer_sku_id.as_deref() != existing.platform_outer_sku_id.as_deref() {
            match repo
                .update_mapping(
                    tenant_id,
                    existing.mapping_id,
                    ChannelSkuMappingUpdate {
                        platform_sku_id: None,
                        platform_outer_sku_id: req.outer_sku_id.clone(),
                        sku_id: None,
                        status: None,
                        conflict_reason: None,
                        confidence: None,
                        match_source: Some("MANUAL".to_string()),
                    },
                )
                .await
            {
                Ok(Some(updated)) => updated,
                Ok(None) => existing,
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
                }
            }
        } else {
            existing
        };

        let view = match mapping_view_from_mapping(state.db.pool(), mapping).await {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        return (StatusCode::OK, Json(view)).into_response();
    }

    let mapping = match repo
        .create_mapping(ChannelSkuMappingCreate {
            tenant_id,
            shop_id,
            platform,
            platform_sku_id: platform_sku_id.to_string(),
            platform_outer_sku_id: req.outer_sku_id.clone(),
            sku_id,
            status: "ACTIVE".to_string(),
            conflict_reason: None,
            confidence: 0,
            match_source: "MANUAL".to_string(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let view = match mapping_view_from_mapping(state.db.pool(), mapping).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (StatusCode::OK, Json(view)).into_response()
}

pub(crate) async fn update_mapping(
    State(state): State<AppState>,
    Path((tenant_id, mapping_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<MappingPayload>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let mapping_id = match uuid::Uuid::parse_str(&mapping_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad mapping_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "channel_sku_mappings.write") {
        return resp.into_response();
    }

    let shop_id = match ShopId::from_str(&req.shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    let platform = match Platform::from_str(&req.platform) {
        Ok(v) => v,
        Err(_) => return bad_request("bad platform").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(&req.sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };
    let platform_sku_id = req.platform_sku_id.trim();
    if platform_sku_id.is_empty() {
        return bad_request("platform_sku_id required").into_response();
    }
    let resolve_conflict = req.resolve_conflict.unwrap_or(false);

    let repo = ChannelSkuMappingsRepo::new(state.db.pool());
    if let Ok(Some(existing)) = repo
        .get_mapping_by_platform_sku(tenant_id, shop_id, platform, platform_sku_id)
        .await
    {
        if existing.mapping_id != mapping_id && existing.sku_id != sku_id {
            let conflict = match build_mapping_conflict(state.db.pool(), &existing, &req.sku_id)
                .await
            {
                Ok(conflict) => conflict,
                Err(err) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response()
                }
            };
            if !resolve_conflict {
                let _ = repo
                    .update_mapping(
                        tenant_id,
                        existing.mapping_id,
                        ChannelSkuMappingUpdate {
                            platform_sku_id: None,
                            platform_outer_sku_id: None,
                            sku_id: None,
                            status: Some("CONFLICT".to_string()),
                            conflict_reason: Some(conflict.conflict_reason.clone()),
                            confidence: None,
                            match_source: None,
                        },
                    )
                    .await;
                return mapping_conflict_response(conflict).into_response();
            }
        }
    }

    let mapping = match repo
        .update_mapping(
            tenant_id,
            mapping_id,
            ChannelSkuMappingUpdate {
                platform_sku_id: Some(platform_sku_id.to_string()),
                platform_outer_sku_id: req.outer_sku_id.clone(),
                sku_id: Some(sku_id),
                status: Some("ACTIVE".to_string()),
                conflict_reason: Some(String::new()),
                confidence: None,
                match_source: Some("MANUAL".to_string()),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("mapping not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let view = match mapping_view_from_mapping(state.db.pool(), mapping).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (StatusCode::OK, Json(view)).into_response()
}

pub(crate) async fn batch_create_mappings(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<MappingBatchRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "channel_sku_mappings.write") {
        return resp.into_response();
    }

    let repo = ChannelSkuMappingsRepo::new(state.db.pool());
    let mut items = Vec::new();
    let mut conflicts = Vec::new();

    for payload in req.items {
        let shop_id = match ShopId::from_str(&payload.shop_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad shop_id").into_response(),
        };
        let platform = match Platform::from_str(&payload.platform) {
            Ok(v) => v,
            Err(_) => return bad_request("bad platform").into_response(),
        };
        let sku_id = match uuid::Uuid::parse_str(&payload.sku_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        let platform_sku_id = payload.platform_sku_id.trim();
        if platform_sku_id.is_empty() {
            return bad_request("platform_sku_id required").into_response();
        }
        let resolve_conflict = payload.resolve_conflict.unwrap_or(false);

        if let Ok(Some(existing)) = repo
            .get_mapping_by_platform_sku(tenant_id, shop_id, platform, platform_sku_id)
            .await
        {
            if existing.sku_id != sku_id {
                if let Ok(conflict) =
                    build_mapping_conflict(state.db.pool(), &existing, &payload.sku_id).await
                {
                    if resolve_conflict {
                        let mapping = match repo
                            .update_mapping(
                                tenant_id,
                                existing.mapping_id,
                                ChannelSkuMappingUpdate {
                                    platform_sku_id: Some(platform_sku_id.to_string()),
                                    platform_outer_sku_id: payload.outer_sku_id.clone(),
                                    sku_id: Some(sku_id),
                                    status: Some("ACTIVE".to_string()),
                                    conflict_reason: Some(String::new()),
                                    confidence: None,
                                    match_source: Some("MANUAL".to_string()),
                                },
                            )
                            .await
                        {
                            Ok(Some(v)) => v,
                            Ok(None) => existing,
                            Err(e) => {
                                return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}"))
                                    .into_response()
                            }
                        };

                        let view = match mapping_view_from_mapping(state.db.pool(), mapping).await {
                            Ok(v) => v,
                            Err(e) => {
                                return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}"))
                                    .into_response()
                            }
                        };
                        items.push(view);
                    } else {
                        let _ = repo
                            .update_mapping(
                                tenant_id,
                                existing.mapping_id,
                                ChannelSkuMappingUpdate {
                                    platform_sku_id: None,
                                    platform_outer_sku_id: None,
                                    sku_id: None,
                                    status: Some("CONFLICT".to_string()),
                                    conflict_reason: Some(conflict.conflict_reason.clone()),
                                    confidence: None,
                                    match_source: None,
                                },
                            )
                            .await;
                        conflicts.push(conflict);
                    }
                }
                continue;
            }

            let mapping = match repo
                .update_mapping(
                    tenant_id,
                    existing.mapping_id,
                    ChannelSkuMappingUpdate {
                        platform_sku_id: Some(platform_sku_id.to_string()),
                        platform_outer_sku_id: payload.outer_sku_id.clone(),
                        sku_id: Some(sku_id),
                        status: Some("ACTIVE".to_string()),
                        conflict_reason: Some(String::new()),
                        confidence: None,
                        match_source: Some("MANUAL".to_string()),
                    },
                )
                .await
            {
                Ok(Some(v)) => v,
                Ok(None) => existing,
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
                }
            };

            let view = match mapping_view_from_mapping(state.db.pool(), mapping).await {
                Ok(v) => v,
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
                }
            };
            items.push(view);
            continue;
        }

        let mapping = match repo
            .create_mapping(ChannelSkuMappingCreate {
                tenant_id,
                shop_id,
                platform,
                platform_sku_id: platform_sku_id.to_string(),
                platform_outer_sku_id: payload.outer_sku_id.clone(),
                sku_id,
                status: "ACTIVE".to_string(),
                conflict_reason: None,
                confidence: 0,
                match_source: "MANUAL".to_string(),
            })
            .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };

        let view = match mapping_view_from_mapping(state.db.pool(), mapping).await {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        items.push(view);
    }

    (
        StatusCode::OK,
        Json(MappingBatchResponse { items, conflicts }),
    )
        .into_response()
}

pub(crate) async fn list_shop_product_skus(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Query(query): Query<ShopProductSkuListQuery>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shop_products.read") {
        return resp.into_response();
    }

    let platform = match parse_platform_optional(query.platform.as_deref()) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let shop_ids = match parse_shop_ids_csv(query.shop_ids.as_deref()) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let updated_from = match parse_datetime_optional(query.updated_from.as_deref(), "updated_from")
    {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let updated_to = match parse_datetime_optional(query.updated_to.as_deref(), "updated_to") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let filters = ShopProductSkuListFilters {
        platform,
        shop_ids,
        product_type: normalize_optional_string(query.product_type),
        status: normalize_optional_string(query.status),
        keyword: normalize_optional_string(query.keyword),
        match_status: normalize_optional_string(query.match_status),
        mapping_status: normalize_optional_string(query.mapping_status),
        conflict_reason: normalize_optional_string(query.conflict_reason),
        match_source: normalize_optional_string(query.match_source),
        updated_from,
        updated_to,
        offset,
        limit,
    };

    let repo = ShopProductsRepo::new(state.db.pool());
    match repo.list_sku_views(tenant_id, filters).await {
        Ok(result) => (
            StatusCode::OK,
            Json(ShopProductSkuListResponse {
                items: result.items,
                total: result.total,
            }),
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    }
}

pub(crate) async fn list_shop_product_tasks(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Query(query): Query<ShopProductTaskListQuery>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shop_products.read") {
        return resp.into_response();
    }

    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let filters = ShopProductTaskListFilters {
        task_type: normalize_optional_string(query.task_type),
        status: normalize_optional_string(query.status),
        offset,
        limit,
    };

    let repo = ShopProductTasksRepo::new(state.db.pool());
    match repo.list_tasks(tenant_id, filters).await {
        Ok(result) => (
            StatusCode::OK,
            Json(ShopProductTaskListResponse {
                items: result.items,
                total: result.total,
            }),
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    }
}

pub(crate) async fn download_shop_products(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ShopProductDownloadRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shop_products.write") {
        return resp.into_response();
    }

    let platform = match parse_platform_optional(req.platform.as_deref()) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let shop_ids = match parse_shop_ids_optional(req.shop_ids.as_deref()) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if platform.is_none() && shop_ids.is_none() {
        return bad_request("platform or shop_ids required").into_response();
    }
    let updated_from = match parse_datetime_optional(req.updated_from.as_deref(), "updated_from") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let updated_to = match parse_datetime_optional(req.updated_to.as_deref(), "updated_to") {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let keyword = normalize_optional_string(req.keyword.clone()).map(|value| value.to_lowercase());
    let desired_status = normalize_optional_string(req.status.clone());
    let desired_product_type = normalize_optional_string(req.product_type.clone());

    let payload_json = serde_json::json!({
        "platform": req.platform,
        "shop_ids": req.shop_ids,
        "product_type": req.product_type,
        "status": req.status,
        "updated_from": req.updated_from,
        "updated_to": req.updated_to,
        "keyword": req.keyword,
        "auto_match": req.auto_match.unwrap_or(false),
    });

    let tasks_repo = ShopProductTasksRepo::new(state.db.pool());
    let mut task = match tasks_repo
        .create_task(ShopProductTaskCreate {
            tenant_id,
            task_type: "DOWNLOAD".to_string(),
            shop_id: None,
            platform,
            payload_json,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let shops_repo = ShopsRepo::new(state.db.pool());
    let shop_filters = RepoShopListFilters {
        platform,
        status: None,
        shop_ids: shop_ids.clone(),
        offset: 0,
        limit: 2000,
    };
    let shop_list = match shops_repo.list_shops(tenant_id, shop_filters).await {
        Ok(v) => v.items,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
    };
    if shop_list.is_empty() {
        let _ = tasks_repo
            .update_task(
                tenant_id,
                task.task_id,
                ShopProductTaskUpdate {
                    status: Some("FAILED".to_string()),
                    total_count: Some(0),
                    success_count: Some(0),
                    failure_count: Some(1),
                    error: Some("no shops found".to_string()),
                    result_json: Some(serde_json::json!({ "errors": ["no shops found"] })),
                    finished_at: Some(Utc::now()),
                },
            )
            .await;
        return bad_request("no shops found").into_response();
    }

    let shop_products_repo = ShopProductsRepo::new(state.db.pool());
    let adapters = AdapterRegistry::new();
    let mut total_count = 0;
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut errors: Vec<String> = Vec::new();
    let mut downloaded_sku_ids: Vec<uuid::Uuid> = Vec::new();

    for shop in shop_list {
        let items = if let Some(items) = req.products.clone() {
            items
        } else {
            let shop_auth = match shops_repo.get_shop_auth(shop.shop_id).await {
                Ok(v) => v,
                Err(e) => {
                    failure_count += 1;
                    errors.push(format!("shop {} auth error: {e}", shop.shop_id));
                    continue;
                }
            };
            let adapter = adapters.platform_adapter(shop.platform);
            let mut cursor = SyncCursor {
                cursor: None,
                last_time: updated_from,
            };
            let mut collected: Vec<ShopProductDownloadItem> = Vec::new();
            let mut round = 0;
            loop {
                round += 1;
                if round > 10 {
                    break;
                }
                let result = match adapter.pull_products(&shop_auth, cursor.clone()).await {
                    Ok(v) => v,
                    Err(e) => {
                        failure_count += 1;
                        errors.push(format!("shop {} pull error: {e}", shop.shop_id));
                        break;
                    }
                };
                let mut converted = build_shop_product_download_items_from_adapter(
                    &result.products,
                    req.product_type.clone(),
                    req.status.clone(),
                );
                collected.append(&mut converted);
                if let Some(next_cursor) = result.next_cursor {
                    cursor.cursor = Some(next_cursor);
                    cursor.last_time = result.latest_time.or(cursor.last_time);
                } else {
                    break;
                }
            }
            collected
        };

        let items = items
            .into_iter()
            .filter(|item| {
                if let Some(product_type) = desired_product_type.as_deref() {
                    if let Some(item_type) = item.product_type.as_deref() {
                        if !item_type.eq_ignore_ascii_case(product_type) {
                            return false;
                        }
                    }
                }
                if let Some(status) = desired_status.as_deref() {
                    if let Some(item_status) = item.status.as_deref() {
                        if !item_status.eq_ignore_ascii_case(status) {
                            return false;
                        }
                    }
                }
                if let Some(keyword) = keyword.as_deref() {
                    let mut matched = item.title.to_lowercase().contains(keyword);
                    if !matched {
                        if let Some(code) = item.merchant_code.as_deref() {
                            matched = code.to_lowercase().contains(keyword);
                        }
                    }
                    if !matched {
                        matched = item.skus.iter().any(|sku| {
                            sku.platform_sku_id.to_lowercase().contains(keyword)
                                || sku
                                    .merchant_sku_code
                                    .as_deref()
                                    .map(|value| value.to_lowercase().contains(keyword))
                                    .unwrap_or(false)
                                || sku
                                    .title
                                    .as_deref()
                                    .map(|value| value.to_lowercase().contains(keyword))
                                    .unwrap_or(false)
                                || sku
                                    .spec_text
                                    .as_deref()
                                    .map(|value| value.to_lowercase().contains(keyword))
                                    .unwrap_or(false)
                        });
                    }
                    if !matched {
                        return false;
                    }
                }
                true
            })
            .collect::<Vec<_>>();

        for item in items {
            let product_type = normalize_optional_string(item.product_type.clone())
                .or_else(|| normalize_optional_string(req.product_type.clone()))
                .unwrap_or_else(|| "STANDARD".to_string());
            let product_status = normalize_optional_string(item.status.clone())
                .or_else(|| normalize_optional_string(req.status.clone()))
                .unwrap_or_else(|| "ACTIVE".to_string());
            let product_updated_at = parse_datetime_best_effort(item.updated_at.as_deref());
            let product_raw = item.raw_json.unwrap_or_else(|| serde_json::json!({}));

            let product = match shop_products_repo
                .upsert_product(ShopProductUpsert {
                    tenant_id,
                    shop_id: shop.shop_id,
                    platform: shop.platform,
                    platform_product_id: item.platform_product_id,
                    merchant_code: normalize_optional_string(item.merchant_code),
                    title: item.title,
                    product_type,
                    status: product_status,
                    platform_updated_at: product_updated_at,
                    raw_json: product_raw,
                })
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    failure_count += 1;
                    errors.push(format!("product upsert failed: {e}"));
                    continue;
                }
            };

            for sku in item.skus {
                if let Some(status) = desired_status.as_deref() {
                    if let Some(sku_status) = sku.status.as_deref() {
                        if !sku_status.eq_ignore_ascii_case(status) {
                            continue;
                        }
                    }
                }
                let sku_updated_at = parse_datetime_best_effort(sku.updated_at.as_deref())
                    .or(product.platform_updated_at);
                if let Some(from) = updated_from {
                    if sku_updated_at.map(|ts| ts < from).unwrap_or(false) {
                        continue;
                    }
                }
                if let Some(to) = updated_to {
                    if sku_updated_at.map(|ts| ts > to).unwrap_or(false) {
                        continue;
                    }
                }

                total_count += 1;
                let sku_status = normalize_optional_string(sku.status)
                    .or_else(|| normalize_optional_string(req.status.clone()))
                    .unwrap_or_else(|| "ACTIVE".to_string());
                let sku_raw = sku.raw_json.unwrap_or_else(|| serde_json::json!({}));
                match shop_products_repo
                    .upsert_sku(ShopProductSkuUpsert {
                        tenant_id,
                        shop_id: shop.shop_id,
                        platform: shop.platform,
                        platform_product_id: product.platform_product_id.clone(),
                        platform_sku_id: sku.platform_sku_id.clone(),
                        merchant_sku_code: normalize_optional_string(sku.merchant_sku_code),
                        title: normalize_optional_string(sku.title),
                        spec_text: normalize_optional_string(sku.spec_text),
                        price_cent: sku.price_cent,
                        stock_qty: sku.stock_qty,
                        status: sku_status,
                        barcode: normalize_optional_string(sku.barcode),
                        image_url: normalize_optional_string(sku.image_url),
                        platform_updated_at: sku_updated_at,
                        raw_json: sku_raw,
                    })
                    .await
                {
                    Ok(record) => {
                        success_count += 1;
                        downloaded_sku_ids.push(record.sku_record_id);
                    }
                    Err(e) => {
                        failure_count += 1;
                        errors.push(format!("sku {} upsert failed: {e}", sku.platform_sku_id));
                    }
                }
            }
        }
    }

    let status = if failure_count > 0 {
        "FAILED"
    } else {
        "SUCCESS"
    };
    let error_message = errors.first().cloned().unwrap_or_default();
    let result_json = serde_json::json!({
        "success_count": success_count,
        "failure_count": failure_count,
        "errors": errors,
    });

    task = match tasks_repo
        .update_task(
            tenant_id,
            task.task_id,
            ShopProductTaskUpdate {
                status: Some(status.to_string()),
                total_count: Some(total_count),
                success_count: Some(success_count),
                failure_count: Some(failure_count),
                error: Some(error_message),
                result_json: Some(result_json),
                finished_at: Some(Utc::now()),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => task,
        Err(_) => task,
    };

    let mut auto_match_task = None;
    let mut auto_match_failures = Vec::new();
    if req.auto_match.unwrap_or(false) && !downloaded_sku_ids.is_empty() {
        let payload_json = serde_json::json!({
            "sku_record_ids": downloaded_sku_ids,
        });
        if let Ok(task_row) = tasks_repo
            .create_task(ShopProductTaskCreate {
                tenant_id,
                task_type: "AUTO_MATCH".to_string(),
                shop_id: None,
                platform,
                payload_json,
            })
            .await
        {
            let sku_ids = dedup_uuid_list(downloaded_sku_ids);
            let views = match shop_products_repo
                .list_sku_views_by_ids(tenant_id, &sku_ids)
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    let _ = tasks_repo
                        .update_task(
                            tenant_id,
                            task_row.task_id,
                            ShopProductTaskUpdate {
                                status: Some("FAILED".to_string()),
                                total_count: Some(0),
                                success_count: Some(0),
                                failure_count: Some(1),
                                error: Some(format!("{e}")),
                                result_json: Some(serde_json::json!({ "errors": [format!("{e}")]})),
                                finished_at: Some(Utc::now()),
                            },
                        )
                        .await;
                    return (
                        StatusCode::OK,
                        Json(ShopProductDownloadResponse {
                            task,
                            auto_match_task: Some(task_row),
                            auto_match_failures,
                        }),
                    )
                        .into_response();
                }
            };
            match auto_match_sku_views(state.db.pool(), tenant_id, &views).await {
                Ok(summary) => {
                    let AutoMatchSummary {
                        total_count,
                        success_count,
                        failure_count,
                        failures,
                    } = summary;
                    let status = if failure_count > 0 {
                        "FAILED"
                    } else {
                        "SUCCESS"
                    };
                    auto_match_failures = failures.clone();
                    let updated = tasks_repo
                        .update_task(
                            tenant_id,
                            task_row.task_id,
                            ShopProductTaskUpdate {
                                status: Some(status.to_string()),
                                total_count: Some(total_count),
                                success_count: Some(success_count),
                                failure_count: Some(failure_count),
                                error: failures.first().map(|item| item.reason.clone()),
                                result_json: Some(serde_json::json!({
                                    "failures": failures,
                                })),
                                finished_at: Some(Utc::now()),
                            },
                        )
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or(task_row);
                    auto_match_task = Some(updated);
                }
                Err(e) => {
                    let _ = tasks_repo
                        .update_task(
                            tenant_id,
                            task_row.task_id,
                            ShopProductTaskUpdate {
                                status: Some("FAILED".to_string()),
                                total_count: Some(0),
                                success_count: Some(0),
                                failure_count: Some(1),
                                error: Some(format!("{e}")),
                                result_json: Some(serde_json::json!({ "errors": [format!("{e}")]})),
                                finished_at: Some(Utc::now()),
                            },
                        )
                        .await;
                    auto_match_task = Some(task_row);
                }
            }
        }
    }

    (
        StatusCode::OK,
        Json(ShopProductDownloadResponse {
            task,
            auto_match_task,
            auto_match_failures,
        }),
    )
        .into_response()
}

pub(crate) async fn create_shop_product_skus(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ShopProductCreateSkuRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shop_products.write") {
        return resp.into_response();
    }

    let sku_record_ids = match parse_uuid_list_optional(req.sku_record_ids.as_deref()) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if sku_record_ids.is_none() && req.filters.is_none() {
        return bad_request("sku_record_ids or filters required").into_response();
    }

    let import_stock = req.import_stock.unwrap_or(false);
    let warehouse_id = if import_stock {
        match req.warehouse_id.as_deref() {
            Some(value) => match uuid::Uuid::parse_str(value) {
                Ok(id) => Some(id),
                Err(_) => return bad_request("bad warehouse_id").into_response(),
            },
            None => return bad_request("warehouse_id required").into_response(),
        }
    } else {
        None
    };

    let filters = req.filters.clone();
    let sku_views =
        match load_shop_product_skus(state.db.pool(), tenant_id, sku_record_ids, filters.clone())
            .await
        {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };

    let tasks_repo = ShopProductTasksRepo::new(state.db.pool());
    let payload_json = serde_json::json!({
        "sku_record_ids": req.sku_record_ids,
        "filters": filters,
        "import_price": req.import_price.unwrap_or(false),
        "import_image": req.import_image.unwrap_or(false),
        "import_stock": import_stock,
        "warehouse_id": req.warehouse_id,
    });
    let mut task = match tasks_repo
        .create_task(ShopProductTaskCreate {
            tenant_id,
            task_type: "CREATE_SKUS".to_string(),
            shop_id: None,
            platform: None,
            payload_json,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let mut total_count = 0;
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut errors: Vec<String> = Vec::new();

    for view in sku_views {
        total_count += 1;
        if view.mapping_id.is_some() {
            failure_count += 1;
            errors.push(format!("sku {} already mapped", view.platform_sku_id));
            continue;
        }

        let spec_text = extract_spec_text(&view);
        let product_name = match spec_text.as_deref() {
            Some(spec) if !spec.is_empty() => format!("{} {}", view.product_title, spec),
            _ => view.product_title.clone(),
        };
        let sku_name = spec_text.clone().or(view.sku_title.clone());
        let base_code = resolve_shop_sku_code(&view);
        let sku_code = match resolve_unique_sku_code(state.db.pool(), tenant_id, &base_code).await {
            Ok(v) => v,
            Err(e) => {
                failure_count += 1;
                errors.push(format!("sku {} code error: {e}", view.platform_sku_id));
                continue;
            }
        };

        let product = match state
            .products_repo
            .create_product(ProductCreate {
                tenant_id,
                product_code: None,
                name: product_name,
                alias: None,
                brand: None,
                category: None,
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
                attrs: serde_json::json!({}),
                allow_negative_stock: false,
            })
            .await
        {
            Ok(v) => v,
            Err(e) => {
                failure_count += 1;
                errors.push(format!("sku {} product error: {e}", view.platform_sku_id));
                continue;
            }
        };

        let mut spec_map = serde_json::Map::new();
        if let Some(spec) = spec_text.clone() {
            spec_map.insert("spec_text".to_string(), Value::String(spec));
        }
        if req.import_price.unwrap_or(false) {
            if let Some(price) = view.price_cent {
                spec_map.insert("shop_price_cent".to_string(), Value::Number(price.into()));
            }
        }
        if let Some(code) = view.merchant_sku_code.clone() {
            spec_map.insert("shop_sku_code".to_string(), Value::String(code));
        }
        let spec_json = Value::Object(spec_map);
        let image_urls = if req.import_image.unwrap_or(false) {
            normalize_string_list(view.image_url.clone().into_iter().collect())
        } else {
            Vec::new()
        };

        let sku = match state
            .skus_repo
            .create_sku(SkuCreate {
                tenant_id,
                product_id: product.product_id,
                sku_code,
                barcode: view.barcode.clone(),
                name: sku_name,
                note: None,
                spec: spec_json,
                tags: Vec::new(),
                image_urls,
                weight_gram: None,
                length_mm: None,
                width_mm: None,
                height_mm: None,
                volume_cm3: None,
                sku_kind: "NORMAL".to_string(),
                cost_cent: None,
                is_active: true,
                status: Some("ACTIVE".to_string()),
                batch_enabled: false,
                serial_enabled: false,
            })
            .await
        {
            Ok(v) => v,
            Err(e) => {
                failure_count += 1;
                errors.push(format!("sku {} create error: {e}", view.platform_sku_id));
                continue;
            }
        };

        let mapping_repo = ChannelSkuMappingsRepo::new(state.db.pool());
        if let Err(e) = mapping_repo
            .create_mapping(ChannelSkuMappingCreate {
                tenant_id,
                shop_id: view.shop_id,
                platform: view.platform,
                platform_sku_id: view.platform_sku_id.clone(),
                platform_outer_sku_id: view.merchant_sku_code.clone(),
                sku_id: sku.sku_id,
                status: "ACTIVE".to_string(),
                conflict_reason: None,
                confidence: 0,
                match_source: "AUTO".to_string(),
            })
            .await
        {
            failure_count += 1;
            errors.push(format!("sku {} mapping error: {e}", view.platform_sku_id));
            continue;
        }

        if import_stock {
            if let Some(qty) = view.stock_qty {
                if let Some(warehouse_id) = warehouse_id {
                    if let Err(e) = state
                        .inventory_repo
                        .adjust(InventoryAdjustment {
                            tenant_id,
                            warehouse_id,
                            sku_id: sku.sku_id,
                            batch_id: None,
                            qty_delta: qty,
                            ref_type: Some("SHOP_PRODUCT_IMPORT".to_string()),
                            ref_id: Some(view.platform_sku_id.clone()),
                        })
                        .await
                    {
                        failure_count += 1;
                        errors.push(format!("sku {} stock error: {e}", view.platform_sku_id));
                        continue;
                    }
                }
            }
        }

        success_count += 1;
    }

    let status = if failure_count > 0 {
        "FAILED"
    } else {
        "SUCCESS"
    };
    let result_json = serde_json::json!({
        "success_count": success_count,
        "failure_count": failure_count,
        "errors": errors,
    });
    task = match tasks_repo
        .update_task(
            tenant_id,
            task.task_id,
            ShopProductTaskUpdate {
                status: Some(status.to_string()),
                total_count: Some(total_count),
                success_count: Some(success_count),
                failure_count: Some(failure_count),
                error: Some(errors.first().cloned().unwrap_or_default()),
                result_json: Some(result_json),
                finished_at: Some(Utc::now()),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => task,
        Err(_) => task,
    };

    (StatusCode::OK, Json(ShopProductTaskResponse { task })).into_response()
}

pub(crate) async fn parse_shop_products(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ShopProductParseRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["shop_products.read", "shop_products.write"]) {
        return resp.into_response();
    }

    let sku_record_ids = match parse_uuid_list_optional(req.sku_record_ids.as_deref()) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if sku_record_ids.is_none() && req.filters.is_none() {
        return bad_request("sku_record_ids or filters required").into_response();
    }

    let sku_views =
        match load_shop_product_skus(state.db.pool(), tenant_id, sku_record_ids, req.filters).await
        {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };

    let mut items = Vec::with_capacity(sku_views.len());
    for view in sku_views {
        if let Some(mapped_sku_id) = view.mapped_sku_id {
            items.push(ShopProductParseItem {
                sku_record_id: view.sku_record_id,
                shop_id: view.shop_id,
                platform: view.platform,
                platform_sku_id: view.platform_sku_id,
                merchant_sku_code: view.merchant_sku_code,
                resolved_sku_id: Some(mapped_sku_id),
                resolved_sku_code: view.mapped_sku_code,
                resolved_sku_name: view.mapped_sku_name,
                parse_source: view.match_source.unwrap_or_else(|| "MAPPING".to_string()),
                reason: None,
            });
            continue;
        }

        let code = resolve_shop_sku_code(&view);
        let mut resolved: Option<MatchedSku> = None;
        let mut parse_source = String::new();

        if !code.is_empty() {
            resolved = match match_code(state.db.pool(), tenant_id, &code).await {
                Ok(v) => v,
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
                }
            };
            if resolved.is_some() {
                parse_source = "CODE".to_string();
            }
        }

        if resolved.is_none() {
            if let Some(merchant_code) = normalize_optional_string(view.merchant_code.clone()) {
                let candidates = build_merged_codes(&merchant_code, &code);
                for candidate in candidates {
                    resolved = match match_code(state.db.pool(), tenant_id, &candidate).await {
                        Ok(v) => v,
                        Err(e) => {
                            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}"))
                                .into_response()
                        }
                    };
                    if resolved.is_some() {
                        parse_source = "BUNDLE_CODE".to_string();
                        break;
                    }
                }
            }
        }

        if resolved.is_none() {
            resolved = match match_by_name_spec(state.db.pool(), tenant_id, &view).await {
                Ok(v) => v,
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
                }
            };
            if resolved.is_some() {
                parse_source = "NAME_SPEC".to_string();
            }
        }

        match resolved {
            Some(found) => items.push(ShopProductParseItem {
                sku_record_id: view.sku_record_id,
                shop_id: view.shop_id,
                platform: view.platform,
                platform_sku_id: view.platform_sku_id,
                merchant_sku_code: view.merchant_sku_code,
                resolved_sku_id: Some(found.sku_id),
                resolved_sku_code: Some(found.sku_code),
                resolved_sku_name: found.name,
                parse_source,
                reason: None,
            }),
            None => items.push(ShopProductParseItem {
                sku_record_id: view.sku_record_id,
                shop_id: view.shop_id,
                platform: view.platform,
                platform_sku_id: view.platform_sku_id,
                merchant_sku_code: view.merchant_sku_code,
                resolved_sku_id: None,
                resolved_sku_code: None,
                resolved_sku_name: None,
                parse_source: "UNRESOLVED".to_string(),
                reason: Some("no sku mapping found".to_string()),
            }),
        }
    }

    (
        StatusCode::OK,
        Json(ShopProductParseResponse {
            total: items.len() as i64,
            items,
        }),
    )
        .into_response()
}

pub(crate) async fn auto_match_shop_products(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ShopProductAutoMatchRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shop_products.write") {
        return resp.into_response();
    }

    let requested_sku_record_ids = req.sku_record_ids.clone();
    let filters = req.filters.clone();
    let sku_record_ids = match parse_uuid_list_optional(requested_sku_record_ids.as_deref()) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if sku_record_ids.is_none() && filters.is_none() {
        return bad_request("sku_record_ids or filters required").into_response();
    }

    let sku_views =
        match load_shop_product_skus(state.db.pool(), tenant_id, sku_record_ids, filters.clone())
            .await
        {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if sku_views.is_empty() {
        return bad_request("no sku records found").into_response();
    }

    let tasks_repo = ShopProductTasksRepo::new(state.db.pool());
    let payload_json = serde_json::json!({
        "sku_record_ids": requested_sku_record_ids,
        "filters": filters.clone(),
    });
    let mut task = match tasks_repo
        .create_task(ShopProductTaskCreate {
            tenant_id,
            task_type: "AUTO_MATCH".to_string(),
            shop_id: None,
            platform: None,
            payload_json,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let summary = match auto_match_sku_views(state.db.pool(), tenant_id, &sku_views).await {
        Ok(v) => v,
        Err(e) => {
            let _ = tasks_repo
                .update_task(
                    tenant_id,
                    task.task_id,
                    ShopProductTaskUpdate {
                        status: Some("FAILED".to_string()),
                        total_count: Some(0),
                        success_count: Some(0),
                        failure_count: Some(1),
                        error: Some(format!("{e}")),
                        result_json: Some(serde_json::json!({ "errors": [format!("{e}")]})),
                        finished_at: Some(Utc::now()),
                    },
                )
                .await;
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
    };

    let AutoMatchSummary {
        total_count,
        success_count,
        failure_count,
        failures,
    } = summary;
    let status = if failure_count > 0 {
        "FAILED"
    } else {
        "SUCCESS"
    };
    task = match tasks_repo
        .update_task(
            tenant_id,
            task.task_id,
            ShopProductTaskUpdate {
                status: Some(status.to_string()),
                total_count: Some(total_count),
                success_count: Some(success_count),
                failure_count: Some(failure_count),
                error: failures.first().map(|item| item.reason.clone()),
                result_json: Some(serde_json::json!({
                    "failures": failures.clone(),
                })),
                finished_at: Some(Utc::now()),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => task,
        Err(_) => task,
    };

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_SHOP_PRODUCT_AUTO_MATCH.to_string(),
            ref_type: Some("SHOP_PRODUCT_TASK".to_string()),
            ref_id: Some(task.task_id.to_string()),
            metadata: Some(serde_json::json!({
                "task_id": task.task_id,
                "task_type": task.task_type,
                "task_status": task.status,
                "requested_sku_record_ids": requested_sku_record_ids,
                "filters": filters,
                "total_count": total_count,
                "success_count": success_count,
                "failure_count": failure_count,
                "failure_reasons": failures.iter().map(|item| item.reason.clone()).collect::<Vec<_>>(),
            })),
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    (
        StatusCode::OK,
        Json(ShopProductMatchResponse { task, failures }),
    )
        .into_response()
}

pub(crate) async fn manual_match_shop_products(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ShopProductManualMatchRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shop_products.write") {
        return resp.into_response();
    }

    let requested_sku_record_ids = req.sku_record_ids.clone();
    let requested_sku_id = req.sku_id.clone();
    let sku_id = match uuid::Uuid::parse_str(&req.sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };
    let sku_exists = match state.skus_repo.get_sku(tenant_id, sku_id).await {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if !sku_exists {
        return not_found("sku not found").into_response();
    }

    let sku_record_ids = match parse_uuid_list(&req.sku_record_ids) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if sku_record_ids.is_empty() {
        return bad_request("sku_record_ids required").into_response();
    }

    let shop_products_repo = ShopProductsRepo::new(state.db.pool());
    let sku_views = match shop_products_repo
        .list_sku_views_by_ids(tenant_id, &sku_record_ids)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if sku_views.is_empty() {
        return bad_request("no sku records found").into_response();
    }

    let tasks_repo = ShopProductTasksRepo::new(state.db.pool());
    let payload_json = serde_json::json!({
        "sku_record_ids": requested_sku_record_ids.clone(),
        "sku_id": requested_sku_id.clone(),
    });
    let mut task = match tasks_repo
        .create_task(ShopProductTaskCreate {
            tenant_id,
            task_type: "MANUAL_MATCH".to_string(),
            shop_id: None,
            platform: None,
            payload_json,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let mapping_repo = ChannelSkuMappingsRepo::new(state.db.pool());
    let mut total_count = 0;
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut errors: Vec<String> = Vec::new();

    for view in sku_views {
        total_count += 1;
        if let Some(mapping_id) = view.mapping_id {
            match mapping_repo
                .update_mapping(
                    tenant_id,
                    mapping_id,
                    ChannelSkuMappingUpdate {
                        platform_sku_id: None,
                        platform_outer_sku_id: view.merchant_sku_code.clone(),
                        sku_id: Some(sku_id),
                        status: Some("ACTIVE".to_string()),
                        conflict_reason: Some(String::new()),
                        confidence: Some(0),
                        match_source: Some("MANUAL".to_string()),
                    },
                )
                .await
            {
                Ok(Some(_)) => {
                    success_count += 1;
                }
                Ok(None) => {
                    failure_count += 1;
                    errors.push(format!("mapping not found: {}", view.platform_sku_id));
                }
                Err(e) => {
                    failure_count += 1;
                    errors.push(format!("update mapping failed: {e}"));
                }
            }
            continue;
        }

        if let Err(e) = mapping_repo
            .create_mapping(ChannelSkuMappingCreate {
                tenant_id,
                shop_id: view.shop_id,
                platform: view.platform,
                platform_sku_id: view.platform_sku_id.clone(),
                platform_outer_sku_id: view.merchant_sku_code.clone(),
                sku_id,
                status: "ACTIVE".to_string(),
                conflict_reason: None,
                confidence: 0,
                match_source: "MANUAL".to_string(),
            })
            .await
        {
            failure_count += 1;
            errors.push(format!("create mapping failed: {e}"));
        } else {
            success_count += 1;
        }
    }

    let status = if failure_count > 0 {
        "FAILED"
    } else {
        "SUCCESS"
    };
    task = match tasks_repo
        .update_task(
            tenant_id,
            task.task_id,
            ShopProductTaskUpdate {
                status: Some(status.to_string()),
                total_count: Some(total_count),
                success_count: Some(success_count),
                failure_count: Some(failure_count),
                error: Some(errors.first().cloned().unwrap_or_default()),
                result_json: Some(serde_json::json!({
                    "errors": errors.clone(),
                })),
                finished_at: Some(Utc::now()),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => task,
        Err(_) => task,
    };

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_SHOP_PRODUCT_MANUAL_MATCH.to_string(),
            ref_type: Some("SHOP_PRODUCT_TASK".to_string()),
            ref_id: Some(task.task_id.to_string()),
            metadata: Some(serde_json::json!({
                "task_id": task.task_id,
                "task_type": task.task_type,
                "task_status": task.status,
                "requested_sku_record_ids": requested_sku_record_ids,
                "requested_sku_id": requested_sku_id,
                "total_count": total_count,
                "success_count": success_count,
                "failure_count": failure_count,
                "errors": errors,
            })),
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    (StatusCode::OK, Json(ShopProductTaskResponse { task })).into_response()
}

pub(crate) async fn auto_match_shop_product_bundles(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ShopProductAutoMatchRequest>,
) -> impl IntoResponse {
    auto_match_shop_products(State(state), Path(tenant_id), Extension(ctx), Json(req)).await
}

pub(crate) async fn manual_match_shop_product_bundles(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ShopProductManualMatchRequest>,
) -> impl IntoResponse {
    manual_match_shop_products(State(state), Path(tenant_id), Extension(ctx), Json(req)).await
}

pub(crate) async fn delete_shop_product_matches(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ShopProductDeleteMatchRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shop_products.write") {
        return resp.into_response();
    }

    let sku_record_ids = match parse_uuid_list(&req.sku_record_ids) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if sku_record_ids.is_empty() {
        return bad_request("sku_record_ids required").into_response();
    }

    let shop_products_repo = ShopProductsRepo::new(state.db.pool());
    let sku_views = match shop_products_repo
        .list_sku_views_by_ids(tenant_id, &sku_record_ids)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if sku_views.is_empty() {
        return bad_request("no sku records found").into_response();
    }

    let tasks_repo = ShopProductTasksRepo::new(state.db.pool());
    let payload_json = serde_json::json!({
        "sku_record_ids": req.sku_record_ids,
    });
    let mut task = match tasks_repo
        .create_task(ShopProductTaskCreate {
            tenant_id,
            task_type: "DELETE_MATCH".to_string(),
            shop_id: None,
            platform: None,
            payload_json,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let mapping_repo = ChannelSkuMappingsRepo::new(state.db.pool());
    let mut total_count = 0;
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut errors: Vec<String> = Vec::new();

    for view in sku_views {
        total_count += 1;
        let Some(mapping_id) = view.mapping_id else {
            failure_count += 1;
            errors.push(format!("sku {} not matched", view.platform_sku_id));
            continue;
        };
        match mapping_repo.delete_mapping(tenant_id, mapping_id).await {
            Ok(Some(_)) => success_count += 1,
            Ok(None) => {
                failure_count += 1;
                errors.push(format!("mapping not found: {}", view.platform_sku_id));
            }
            Err(e) => {
                failure_count += 1;
                errors.push(format!("delete mapping failed: {e}"));
            }
        }
    }

    let status = if failure_count > 0 {
        "FAILED"
    } else {
        "SUCCESS"
    };
    task = match tasks_repo
        .update_task(
            tenant_id,
            task.task_id,
            ShopProductTaskUpdate {
                status: Some(status.to_string()),
                total_count: Some(total_count),
                success_count: Some(success_count),
                failure_count: Some(failure_count),
                error: Some(errors.first().cloned().unwrap_or_default()),
                result_json: Some(serde_json::json!({
                    "errors": errors,
                })),
                finished_at: Some(Utc::now()),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => task,
        Err(_) => task,
    };

    (StatusCode::OK, Json(ShopProductTaskResponse { task })).into_response()
}

pub(crate) async fn create_channel_sku_mapping(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ChannelSkuMappingCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "channel_sku_mappings.write") {
        return resp.into_response();
    }

    let shop_id = match ShopId::from_str(&req.shop_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    let platform = match Platform::from_str(&req.platform) {
        Ok(v) => v,
        Err(_) => return bad_request("bad platform").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(&req.sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };
    let match_source = req
        .match_source
        .as_deref()
        .unwrap_or("MANUAL")
        .trim()
        .to_uppercase();

    let mapping = match state
        .channel_sku_mappings_repo
        .create_mapping(ChannelSkuMappingCreate {
            tenant_id,
            shop_id,
            platform,
            platform_sku_id: req.platform_sku_id,
            platform_outer_sku_id: req.platform_outer_sku_id,
            sku_id,
            status: req.status.unwrap_or_else(|| "ACTIVE".to_string()),
            conflict_reason: None,
            confidence: req.confidence.unwrap_or(0),
            match_source,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(mapping)).into_response()
}

pub(crate) async fn update_channel_sku_mapping(
    State(state): State<AppState>,
    Path((tenant_id, mapping_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ChannelSkuMappingUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "channel_sku_mappings.write") {
        return resp.into_response();
    }

    let mapping_id = match uuid::Uuid::parse_str(&mapping_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad mapping_id").into_response(),
    };

    let sku_id = match req.sku_id {
        Some(v) => match uuid::Uuid::parse_str(&v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };
    let conflict_reason = req.conflict_reason.map(|value| value.trim().to_string());

    let mapping = match state
        .channel_sku_mappings_repo
        .update_mapping(
            tenant_id,
            mapping_id,
            ChannelSkuMappingUpdate {
                platform_sku_id: req.platform_sku_id,
                platform_outer_sku_id: req.platform_outer_sku_id,
                sku_id,
                status: req.status,
                conflict_reason,
                confidence: req.confidence,
                match_source: req.match_source.map(|value| value.trim().to_uppercase()),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("mapping not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(mapping)).into_response()
}

pub(crate) async fn list_channel_field_mappings(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ChannelFieldMappingListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "channel_sku_mappings.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let result = match ChannelFieldMappingsRepo::new(state.db.pool())
        .list_mappings(
            tenant_id,
            ChannelFieldMappingListFilters {
                platform: query.platform.and_then(non_empty),
                entity_type: query.entity_type.and_then(non_empty),
                platform_field: query.platform_field.and_then(non_empty),
                internal_field: query.internal_field.and_then(non_empty),
                enabled: query.enabled,
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
        Json(ChannelFieldMappingListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn create_channel_field_mapping(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ChannelFieldMappingCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "channel_sku_mappings.write") {
        return resp.into_response();
    }
    if req.platform.trim().is_empty() {
        return bad_request("platform required").into_response();
    }
    if req.entity_type.trim().is_empty() {
        return bad_request("entity_type required").into_response();
    }
    if req.platform_field.trim().is_empty() {
        return bad_request("platform_field required").into_response();
    }
    if req.internal_field.trim().is_empty() {
        return bad_request("internal_field required").into_response();
    }

    let mapping = match ChannelFieldMappingsRepo::new(state.db.pool())
        .create_mapping(ChannelFieldMappingCreate {
            tenant_id,
            platform: req.platform.trim().to_string(),
            entity_type: req.entity_type.trim().to_string(),
            platform_field: req.platform_field.trim().to_string(),
            internal_field: req.internal_field.trim().to_string(),
            default_value: req.default_value.and_then(non_empty),
            transform_rule: req.transform_rule.unwrap_or_else(|| serde_json::json!({})),
            enabled: req.enabled.unwrap_or(true),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "CHANNEL_FIELD_MAPPINGS.CREATE",
        "CHANNEL_FIELD_MAPPING",
        Some(mapping.mapping_id.to_string()),
        None,
        Some(serde_json::json!({
            "platform": mapping.platform,
            "entity_type": mapping.entity_type,
            "platform_field": mapping.platform_field,
            "internal_field": mapping.internal_field,
            "enabled": mapping.enabled,
        })),
    )
    .await;

    (StatusCode::OK, Json(mapping)).into_response()
}

pub(crate) async fn update_channel_field_mapping(
    State(state): State<AppState>,
    Path((tenant_id, mapping_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ChannelFieldMappingUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let mapping_id = match uuid::Uuid::parse_str(&mapping_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad mapping_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "channel_sku_mappings.write") {
        return resp.into_response();
    }

    let mapping = match ChannelFieldMappingsRepo::new(state.db.pool())
        .update_mapping(
            tenant_id,
            mapping_id,
            ChannelFieldMappingUpdate {
                platform: req.platform.and_then(non_empty),
                entity_type: req.entity_type.and_then(non_empty),
                platform_field: req.platform_field.and_then(non_empty),
                internal_field: req.internal_field.and_then(non_empty),
                default_value: req.default_value.map(|value| value.and_then(non_empty)),
                transform_rule: req.transform_rule,
                enabled: req.enabled,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("mapping not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "CHANNEL_FIELD_MAPPINGS.UPDATE",
        "CHANNEL_FIELD_MAPPING",
        Some(mapping.mapping_id.to_string()),
        None,
        Some(serde_json::json!({
            "platform": mapping.platform,
            "entity_type": mapping.entity_type,
            "platform_field": mapping.platform_field,
            "internal_field": mapping.internal_field,
            "enabled": mapping.enabled,
        })),
    )
    .await;

    (StatusCode::OK, Json(mapping)).into_response()
}

pub(crate) async fn delete_channel_field_mapping(
    State(state): State<AppState>,
    Path((tenant_id, mapping_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let mapping_id = match uuid::Uuid::parse_str(&mapping_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad mapping_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "channel_sku_mappings.write") {
        return resp.into_response();
    }

    let mapping = match ChannelFieldMappingsRepo::new(state.db.pool())
        .delete_mapping(tenant_id, mapping_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("mapping not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "CHANNEL_FIELD_MAPPINGS.DELETE",
        "CHANNEL_FIELD_MAPPING",
        Some(mapping.mapping_id.to_string()),
        None,
        Some(serde_json::json!({
            "platform": mapping.platform,
            "entity_type": mapping.entity_type,
            "platform_field": mapping.platform_field,
            "internal_field": mapping.internal_field,
            "enabled": mapping.enabled,
        })),
    )
    .await;

    (StatusCode::OK, Json(mapping)).into_response()
}

pub(crate) async fn list_mapping_rules(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<MappingRuleListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "mapping_rules.read") {
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
    let entity_type = query
        .entity_type
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let enabled = query.enabled;
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = MappingRulesRepo::new(state.db.pool());
    let result = match repo
        .list_rules(
            tenant_id,
            MappingRuleListFilters {
                platform,
                entity_type,
                enabled,
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
        Json(MappingRuleListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn create_mapping_rule(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<MappingRuleCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "mapping_rules.write") {
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
    let entity_type = req.entity_type.trim();
    if entity_type.is_empty() {
        return bad_request("entity_type is required").into_response();
    }
    if req.version <= 0 {
        return bad_request("version must be positive").into_response();
    }
    let rule_json = req.rule_json.unwrap_or_else(|| serde_json::json!({}));

    let repo = MappingRulesRepo::new(state.db.pool());
    let rule = match repo
        .create_rule(MappingRuleCreate {
            tenant_id,
            platform,
            entity_type: entity_type.to_string(),
            version: req.version,
            enabled: req.enabled,
            rule_json,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => {
            let message = e.to_string();
            if message.contains("invalid mapping rule:") {
                return bad_request(&message).into_response();
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, message).into_response();
        }
    };

    (StatusCode::OK, Json(rule)).into_response()
}

pub(crate) async fn update_mapping_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<MappingRuleUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "mapping_rules.write") {
        return resp.into_response();
    }
    if req.enabled.is_none() && req.rule_json.is_none() {
        return bad_request("no fields to update").into_response();
    }

    let repo = MappingRulesRepo::new(state.db.pool());
    let rule = match repo
        .update_rule(
            tenant_id,
            rule_id,
            MappingRuleUpdate {
                enabled: req.enabled,
                rule_json: req.rule_json,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("mapping rule not found").into_response(),
        Err(e) => {
            let message = e.to_string();
            if message.contains("invalid mapping rule:") {
                return bad_request(&message).into_response();
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, message).into_response();
        }
    };

    (StatusCode::OK, Json(rule)).into_response()
}
