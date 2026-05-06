use super::*;

#[derive(Debug)]
pub(crate) struct AutoMatchSummary {
    pub(crate) total_count: i32,
    pub(crate) success_count: i32,
    pub(crate) failure_count: i32,
    pub(crate) failures: Vec<ShopProductMatchFailure>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct MatchedSku {
    pub(crate) sku_id: uuid::Uuid,
    pub(crate) sku_code: String,
    pub(crate) name: Option<String>,
    barcode: Option<String>,
    spec: Value,
}

pub(crate) async fn auto_match_sku_views(
    pool: &PgPool,
    tenant_id: TenantId,
    views: &[ShopProductSkuView],
) -> anyhow::Result<AutoMatchSummary> {
    let mapping_repo = ChannelSkuMappingsRepo::new(pool);
    let mut total_count = 0;
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut failures = Vec::new();

    for view in views {
        total_count += 1;
        if view.mapping_id.is_some() {
            if view
                .match_source
                .as_deref()
                .map(|value| value.eq_ignore_ascii_case("MANUAL"))
                .unwrap_or(false)
            {
                failure_count += 1;
                failures.push(ShopProductMatchFailure {
                    sku_record_id: view.sku_record_id,
                    platform_sku_id: view.platform_sku_id.clone(),
                    merchant_sku_code: view.merchant_sku_code.clone(),
                    reason: "已存在手动匹配".to_string(),
                });
            } else {
                success_count += 1;
            }
            continue;
        }

        let code = resolve_shop_sku_code(view);
        let mut matched = None;
        if !code.is_empty() {
            matched = match_code(pool, tenant_id, &code).await?;
        }

        let mut reason = if matched.is_none() {
            Some("未找到条码、辅助条码或编号对应货品".to_string())
        } else {
            None
        };

        if matched.is_none() {
            if let Some(merchant_code) = normalize_optional_string(view.merchant_code.clone()) {
                let candidates = build_merged_codes(&merchant_code, &code);
                for candidate in candidates {
                    matched = match_code(pool, tenant_id, &candidate).await?;
                    if matched.is_some() {
                        break;
                    }
                }
            }
        }

        if matched.is_none() {
            if let Some(name_match) = match_by_name_spec(pool, tenant_id, view).await? {
                matched = Some(name_match);
                reason = None;
            } else if view.product_title.trim().is_empty() {
                reason = Some("未找到条码、辅助条码或编号对应货品".to_string());
            } else {
                reason = Some("未找到名称规格对应货品".to_string());
            }
        }

        let Some(matched) = matched else {
            failure_count += 1;
            failures.push(ShopProductMatchFailure {
                sku_record_id: view.sku_record_id,
                platform_sku_id: view.platform_sku_id.clone(),
                merchant_sku_code: view.merchant_sku_code.clone(),
                reason: reason.unwrap_or_else(|| "自动匹配失败".to_string()),
            });
            continue;
        };

        match mapping_repo
            .create_mapping(ChannelSkuMappingCreate {
                tenant_id,
                shop_id: view.shop_id,
                platform: view.platform,
                platform_sku_id: view.platform_sku_id.clone(),
                platform_outer_sku_id: view.merchant_sku_code.clone(),
                sku_id: matched.sku_id,
                status: "ACTIVE".to_string(),
                conflict_reason: None,
                confidence: 0,
                match_source: "AUTO".to_string(),
            })
            .await
        {
            Ok(_) => success_count += 1,
            Err(e) => {
                failure_count += 1;
                failures.push(ShopProductMatchFailure {
                    sku_record_id: view.sku_record_id,
                    platform_sku_id: view.platform_sku_id.clone(),
                    merchant_sku_code: view.merchant_sku_code.clone(),
                    reason: format!("匹配写入失败: {e}"),
                });
            }
        }
    }

    Ok(AutoMatchSummary {
        total_count,
        success_count,
        failure_count,
        failures,
    })
}

pub(crate) async fn match_code(
    pool: &PgPool,
    tenant_id: TenantId,
    code: &str,
) -> anyhow::Result<Option<MatchedSku>> {
    if let Some(found) = find_sku_by_barcode(pool, tenant_id, code).await? {
        return Ok(Some(found));
    }
    if let Some(found) = find_sku_by_aux_barcode(pool, tenant_id, code).await? {
        return Ok(Some(found));
    }
    if let Some(found) = find_sku_by_sku_code(pool, tenant_id, code).await? {
        return Ok(Some(found));
    }
    Ok(None)
}

pub(crate) async fn find_sku_by_barcode(
    pool: &PgPool,
    tenant_id: TenantId,
    code: &str,
) -> anyhow::Result<Option<MatchedSku>> {
    let code = code.trim();
    if code.is_empty() {
        return Ok(None);
    }
    #[derive(sqlx::FromRow)]
    struct Row {
        sku_id: uuid::Uuid,
        sku_code: String,
        name: Option<String>,
        barcode: Option<String>,
        spec: Value,
    }
    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select sku_id, sku_code, name, barcode, spec
        from skus
        where tenant_id = $1 and status <> 'DELETED' and barcode = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(code)
    .fetch_all(pool)
    .await?;
    if rows.len() == 1 {
        let row = &rows[0];
        Ok(Some(MatchedSku {
            sku_id: row.sku_id,
            sku_code: row.sku_code.clone(),
            name: row.name.clone(),
            barcode: row.barcode.clone(),
            spec: row.spec.clone(),
        }))
    } else {
        Ok(None)
    }
}

pub(crate) async fn find_sku_by_aux_barcode(
    pool: &PgPool,
    tenant_id: TenantId,
    code: &str,
) -> anyhow::Result<Option<MatchedSku>> {
    let code = code.trim();
    if code.is_empty() {
        return Ok(None);
    }
    #[derive(sqlx::FromRow)]
    struct Row {
        sku_id: uuid::Uuid,
        sku_code: String,
        name: Option<String>,
        barcode: Option<String>,
        spec: Value,
    }
    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select sku_id, sku_code, name, barcode, spec
        from skus
        where tenant_id = $1
          and status <> 'DELETED'
          and (
            spec->>'aux_barcode' = $2
            or spec->>'assistant_barcode' = $2
          )
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(code)
    .fetch_all(pool)
    .await?;
    if rows.len() == 1 {
        let row = &rows[0];
        Ok(Some(MatchedSku {
            sku_id: row.sku_id,
            sku_code: row.sku_code.clone(),
            name: row.name.clone(),
            barcode: row.barcode.clone(),
            spec: row.spec.clone(),
        }))
    } else {
        Ok(None)
    }
}

pub(crate) async fn find_sku_by_sku_code(
    pool: &PgPool,
    tenant_id: TenantId,
    code: &str,
) -> anyhow::Result<Option<MatchedSku>> {
    let code = code.trim();
    if code.is_empty() {
        return Ok(None);
    }
    #[derive(sqlx::FromRow)]
    struct Row {
        sku_id: uuid::Uuid,
        sku_code: String,
        name: Option<String>,
        barcode: Option<String>,
        spec: Value,
    }
    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select sku_id, sku_code, name, barcode, spec
        from skus
        where tenant_id = $1 and status <> 'DELETED' and sku_code = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(code)
    .fetch_all(pool)
    .await?;
    if rows.len() == 1 {
        let row = &rows[0];
        Ok(Some(MatchedSku {
            sku_id: row.sku_id,
            sku_code: row.sku_code.clone(),
            name: row.name.clone(),
            barcode: row.barcode.clone(),
            spec: row.spec.clone(),
        }))
    } else {
        Ok(None)
    }
}

pub(crate) async fn match_by_name_spec(
    pool: &PgPool,
    tenant_id: TenantId,
    view: &ShopProductSkuView,
) -> anyhow::Result<Option<MatchedSku>> {
    let product_name = view.product_title.trim();
    if product_name.is_empty() {
        return Ok(None);
    }
    let target_spec = extract_spec_text(view).map(|value| value.to_lowercase());

    #[derive(sqlx::FromRow)]
    struct Row {
        sku_id: uuid::Uuid,
        sku_code: String,
        name: Option<String>,
        barcode: Option<String>,
        spec: Value,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select s.sku_id, s.sku_code, s.name, s.barcode, s.spec
        from skus s
        join products p
          on p.tenant_id = s.tenant_id
         and p.product_id = s.product_id
        where s.tenant_id = $1 and s.status <> 'DELETED' and lower(p.name) = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(product_name.to_lowercase())
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(None);
    }

    let mut matched_rows = Vec::new();
    for row in rows {
        if let Some(target_spec) = target_spec.as_ref() {
            let candidate = extract_spec_text_from_spec(&row.spec)
                .map(|value| value.to_lowercase())
                .unwrap_or_default();
            if candidate == *target_spec {
                matched_rows.push(row);
            }
        } else {
            matched_rows.push(row);
        }
    }

    if matched_rows.len() == 1 {
        let row = matched_rows.remove(0);
        Ok(Some(MatchedSku {
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            name: row.name,
            barcode: row.barcode,
            spec: row.spec,
        }))
    } else {
        Ok(None)
    }
}

pub(crate) fn extract_spec_text(view: &ShopProductSkuView) -> Option<String> {
    normalize_optional_string(view.spec_text.clone())
        .or_else(|| normalize_optional_string(view.sku_title.clone()))
}

pub(crate) fn resolve_shop_sku_code(view: &ShopProductSkuView) -> String {
    normalize_optional_string(view.merchant_sku_code.clone())
        .unwrap_or_else(|| view.platform_sku_id.clone())
}

pub(crate) fn extract_spec_text_from_spec(spec: &Value) -> Option<String> {
    match spec {
        Value::Object(map) => map
            .get("spec_text")
            .or_else(|| map.get("spec_value"))
            .or_else(|| map.get("spec"))
            .and_then(|value| value.as_str().map(|v| v.to_string())),
        Value::String(value) => Some(value.clone()),
        _ => None,
    }
}

pub(crate) fn build_merged_codes(product_code: &str, sku_code: &str) -> Vec<String> {
    let mut values = Vec::new();
    if !product_code.trim().is_empty() && !sku_code.trim().is_empty() {
        values.push(format!("{}{}", product_code.trim(), sku_code.trim()));
        values.push(format!("{}-{}", product_code.trim(), sku_code.trim()));
    }
    values
}

pub(crate) async fn load_shop_product_skus(
    pool: &PgPool,
    tenant_id: TenantId,
    sku_record_ids: Option<Vec<uuid::Uuid>>,
    filters: Option<ShopProductSkuFilterPayload>,
) -> Result<Vec<ShopProductSkuView>, Response> {
    let repo = ShopProductsRepo::new(pool);
    if let Some(ids) = sku_record_ids {
        return repo
            .list_sku_views_by_ids(tenant_id, &ids)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
    }

    let Some(filters) = filters else {
        return Ok(Vec::new());
    };
    let platform = parse_platform_optional(filters.platform.as_deref())
        .map_err(|resp| resp.into_response())?;
    let shop_ids = parse_shop_ids_optional(filters.shop_ids.as_deref())
        .map_err(|resp| resp.into_response())?;
    let updated_from = parse_datetime_optional(filters.updated_from.as_deref(), "updated_from")
        .map_err(|resp| resp.into_response())?;
    let updated_to = parse_datetime_optional(filters.updated_to.as_deref(), "updated_to")
        .map_err(|resp| resp.into_response())?;
    let mut offset = 0;
    let limit = 200;
    let mut items = Vec::new();
    loop {
        let result = repo
            .list_sku_views(
                tenant_id,
                ShopProductSkuListFilters {
                    platform,
                    shop_ids: shop_ids.clone(),
                    product_type: normalize_optional_string(filters.product_type.clone()),
                    status: normalize_optional_string(filters.status.clone()),
                    keyword: normalize_optional_string(filters.keyword.clone()),
                    match_status: normalize_optional_string(filters.match_status.clone()),
                    mapping_status: normalize_optional_string(filters.mapping_status.clone()),
                    conflict_reason: normalize_optional_string(filters.conflict_reason.clone()),
                    match_source: normalize_optional_string(filters.match_source.clone()),
                    updated_from,
                    updated_to,
                    offset,
                    limit,
                },
            )
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response())?;
        items.extend(result.items);
        offset += limit;
        if offset >= result.total {
            break;
        }
    }
    Ok(items)
}

pub(crate) async fn resolve_unique_sku_code(
    pool: &PgPool,
    tenant_id: TenantId,
    base_code: &str,
) -> anyhow::Result<String> {
    let base = base_code.trim();
    let base_value = if base.is_empty() {
        format!("SHOP-{}", uuid::Uuid::new_v4())
    } else {
        base.to_string()
    };
    let mut candidate = base_value.clone();
    let mut index = 1;
    while sku_code_exists(pool, tenant_id, &candidate).await? {
        candidate = format!("{base_value}-{index}");
        index += 1;
    }
    Ok(candidate)
}

pub(crate) async fn sku_code_exists(
    pool: &PgPool,
    tenant_id: TenantId,
    sku_code: &str,
) -> anyhow::Result<bool> {
    let exists: Option<i32> = sqlx::query_scalar(
        r#"
        select 1
        from skus
        where tenant_id = $1 and sku_code = $2 and status <> 'DELETED'
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(sku_code)
    .fetch_optional(pool)
    .await?;
    Ok(exists.is_some())
}
