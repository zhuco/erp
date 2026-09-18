use super::*;

pub(crate) async fn request_context_middleware(
    mut req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> Response {
    let headers = req.headers();
    let request_id = headers
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(generate_trace_id);
    let trace_id = headers
        .get(TRACE_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| request_id.clone());

    let ip = extract_ip(headers);
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string());

    req.extensions_mut().insert(RequestContext {
        request_id: request_id.clone(),
        trace_id: trace_id.clone(),
        ip,
        user_agent,
    });

    let mut response = next.run(req).await;
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(REQUEST_ID_HEADER, value);
    }
    if let Ok(value) = HeaderValue::from_str(&trace_id) {
        response.headers_mut().insert(TRACE_ID_HEADER, value);
    }
    response
}

fn extract_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some((first, _)) = value.split_once(',') {
            let ip = first.trim();
            if !ip.is_empty() {
                return Some(ip.to_string());
            }
        } else if !value.trim().is_empty() {
            return Some(value.trim().to_string());
        }
    }
    headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub(crate) async fn auth_middleware(
    State(state): State<AppState>,
    mut req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> impl IntoResponse {
    let path = req.uri().path().to_string();
    if path == "/healthz" || path == "/auth/login" || path.starts_with("/v1/print-agent") {
        return next.run(req).await;
    }

    let headers = req.headers();
    let Some(auth) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    else {
        return (StatusCode::UNAUTHORIZED, "missing Authorization").into_response();
    };
    let Some(token) = auth
        .strip_prefix("Bearer ")
        .or_else(|| auth.strip_prefix("bearer "))
    else {
        return (StatusCode::UNAUTHORIZED, "invalid Authorization scheme").into_response();
    };

    match state.jwt.decode(token) {
        Ok(claims) => {
            let tenant_id = match TenantId::from_str(&claims.tenant_id) {
                Ok(v) => v,
                Err(_) => {
                    return (StatusCode::UNAUTHORIZED, "bad tenant_id in token").into_response();
                }
            };
            let user_id = match UserId::from_str(&claims.sub) {
                Ok(v) => v,
                Err(_) => return (StatusCode::UNAUTHORIZED, "bad sub in token").into_response(),
            };
            if !claims.is_admin {
                if let Some(path_tenant_id) = tenant_id_from_path(&path) {
                    if path_tenant_id != tenant_id {
                        return forbidden("tenant mismatch").into_response();
                    }
                }
            }
            let perms = normalize_permissions(claims.perms);
            let (shop_ids, warehouse_ids, merchant_ids, organization_ids) = if claims.is_admin {
                (Vec::new(), Vec::new(), Vec::new(), Vec::new())
            } else {
                let scopes_repo = DataScopesRepo::new(state.db.pool());
                match scopes_repo.list_user_scopes(tenant_id, user_id).await {
                    Ok(scopes) => {
                        let tenants_repo = TenantsRepo::new(state.db.pool());
                        let (multi_merchant_enabled, multi_organization_enabled) =
                            match tenants_repo.multi_entity_flags(tenant_id).await {
                                Ok(flags) => flags,
                                Err(err) => {
                                    warn!(
                                        tenant_id=%tenant_id,
                                        error=%err,
                                        "failed to load multi-entity flags; disabling scopes"
                                    );
                                    (false, false)
                                }
                            };
                        let merchant_ids = if multi_merchant_enabled {
                            scopes.merchant_ids
                        } else {
                            Vec::new()
                        };
                        let organization_ids = if multi_organization_enabled {
                            scopes.organization_ids
                        } else {
                            Vec::new()
                        };
                        (
                            scopes.shop_ids,
                            scopes.warehouse_ids,
                            merchant_ids,
                            organization_ids,
                        )
                    }
                    Err(err) => {
                        warn!(tenant_id=%tenant_id, user_id=%user_id, error=%err, "failed to load data scopes");
                        (Vec::new(), Vec::new(), Vec::new(), Vec::new())
                    }
                }
            };
            req.extensions_mut().insert(AuthContext {
                tenant_id,
                user_id,
                perms,
                is_admin: claims.is_admin,
                shop_ids,
                warehouse_ids,
                merchant_ids,
                organization_ids,
            });
            next.run(req).await
        }
        Err(e) => {
            warn!(error = %e, "jwt decode failed");
            (StatusCode::UNAUTHORIZED, "invalid token").into_response()
        }
    }
}

pub(crate) async fn device_auth_middleware(
    State(state): State<AppState>,
    mut req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> impl IntoResponse {
    let headers = req.headers();
    let token = headers
        .get("x-device-token")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(token) = token else {
        return (StatusCode::UNAUTHORIZED, "missing device token").into_response();
    };

    let repo = PrintDevicesRepoCore::new(state.db.pool());
    let device = match repo.get_device_by_token(token).await {
        Ok(Some(device)) => device,
        Ok(None) => return (StatusCode::UNAUTHORIZED, "invalid device token").into_response(),
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("device lookup failed: {err}"),
            )
                .into_response();
        }
    };

    if device.status != "ACTIVE" {
        return (StatusCode::FORBIDDEN, "device inactive").into_response();
    }

    req.extensions_mut().insert(DeviceAuthContext {
        tenant_id: device.tenant_id,
        device_id: device.device_id,
        fingerprint: device.fingerprint,
        name: device.name,
    });
    next.run(req).await
}

// POST carries a structured read query; match only this exact route and method.
pub(crate) fn is_product_grid_read(method: &axum::http::Method, path: &str) -> bool {
    let parts: Vec<_> = path.split('/').collect();
    method == axum::http::Method::POST
        && (matches!(parts.as_slice(), ["", "v1", "tenants", tenant, "products", "query" | "options" | "find"]
            if TenantId::from_str(tenant).is_ok())
            || matches!(parts.as_slice(), ["", "v1", "tenants", tenant, "products", product, "skus", "query" | "query-ranges" | "find"]
                if TenantId::from_str(tenant).is_ok() && uuid::Uuid::parse_str(product).is_ok())
            || matches!(parts.as_slice(), ["", "v1", "tenants", tenant, "products", "selections"]
                if TenantId::from_str(tenant).is_ok())
            || matches!(parts.as_slice(), ["", "v1", "tenants", tenant, "products", "selections", token, "page" | "count" | "find"]
                if TenantId::from_str(tenant).is_ok() && uuid::Uuid::parse_str(token).is_ok()))
}

// POST carries a bounded report query; match only this exact tenant route and method.
pub(crate) fn is_inventory_aggregate_read(method: &axum::http::Method, path: &str) -> bool {
    let parts: Vec<_> = path.split('/').collect();
    method == axum::http::Method::POST
        && matches!(
            parts.as_slice(),
            ["", "v1", "tenants", tenant, "reports", "inventory", "aggregate"]
                if TenantId::from_str(tenant).is_ok()
                    && uuid::Uuid::parse_str(tenant).is_ok_and(|id| !id.is_nil())
        )
}

pub(crate) fn is_warehouse_tree_read(method: &axum::http::Method, path: &str) -> bool {
    *method == axum::http::Method::POST
        && matches!(
            path.split('/').collect::<Vec<_>>().as_slice(),
            ["", "v1", "tenants", tenant, "warehouses", "tree"]
                if uuid::Uuid::parse_str(tenant).is_ok_and(|id| !id.is_nil())
        )
}

pub(crate) async fn read_only_middleware(
    State(state): State<AppState>,
    req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> impl IntoResponse {
    if !is_write_method(req.method()) {
        return next.run(req).await;
    }

    let path = req.uri().path();
    if path == "/healthz"
        || path == "/auth/login"
        || is_product_grid_read(req.method(), path)
        || is_inventory_aggregate_read(req.method(), path)
        || is_warehouse_tree_read(req.method(), path)
    {
        return next.run(req).await;
    }

    let Some(tenant_id) = tenant_id_from_path(path) else {
        return next.run(req).await;
    };

    let read_only = match read_only_enabled(&state, tenant_id, None).await {
        Ok(v) => v,
        Err(resp) => {
            let parts: Vec<_> = path.split('/').collect();
            if matches!(
                parts.as_slice(),
                ["", "v1", "tenants", _, "products", "actions"]
            ) && req.method() == axum::http::Method::POST
            {
                // New grid mutations must not inherit the legacy fail-open path.
                return resp.into_response();
            }
            warn!(
                tenant_id = %tenant_id,
                status = %resp.0,
                "read-only check failed; allowing request"
            );
            return next.run(req).await;
        }
    };
    if read_only {
        return read_only_forbidden().into_response();
    }

    next.run(req).await
}

pub(crate) async fn healthz(State(state): State<AppState>) -> impl IntoResponse {
    // 深度健康检查：数据库 ping
    let ok = sqlx::query_scalar::<_, i64>("select 1::bigint")
        .fetch_one(state.db.pool())
        .await
        .is_ok();

    if ok {
        (StatusCode::OK, "ok")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "db not ready")
    }
}
