use super::*;

pub(crate) async fn fetch_existing_sku_codes(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    sku_codes: &[String],
) -> Result<HashSet<String>, (StatusCode, Json<ErrorResponse>)> {
    if sku_codes.is_empty() {
        return Ok(HashSet::new());
    }
    let rows: Vec<String> = sqlx::query_scalar(
        r#"
        select sku_code
        from skus
        where tenant_id = $1
          and status <> 'DELETED'
          and sku_code = any($2)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(sku_codes)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                code: "internal_error".to_string(),
                message: format!("failed to load sku codes: {e}"),
                details: None,
                request_id: None,
            }),
        )
    })?;
    Ok(rows.into_iter().collect())
}

pub(crate) async fn fetch_existing_product_ids(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    product_ids: &[uuid::Uuid],
) -> Result<HashSet<uuid::Uuid>, (StatusCode, Json<ErrorResponse>)> {
    if product_ids.is_empty() {
        return Ok(HashSet::new());
    }
    let rows: Vec<uuid::Uuid> = sqlx::query_scalar(
        r#"
        select product_id
        from products
        where tenant_id = $1
          and status <> 'DELETED'
          and product_id = any($2)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(product_ids)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                code: "internal_error".to_string(),
                message: format!("failed to load products: {e}"),
                details: None,
                request_id: None,
            }),
        )
    })?;
    Ok(rows.into_iter().collect())
}

pub(crate) async fn fetch_product_categories_by_id(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    product_ids: &[uuid::Uuid],
) -> Result<HashMap<uuid::Uuid, Option<String>>, (StatusCode, Json<ErrorResponse>)> {
    if product_ids.is_empty() {
        return Ok(HashMap::new());
    }
    #[derive(sqlx::FromRow)]
    struct Row {
        product_id: uuid::Uuid,
        category: Option<String>,
    }
    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select product_id, category
        from products
        where tenant_id = $1
          and status <> 'DELETED'
          and product_id = any($2)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(product_ids)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                code: "internal_error".to_string(),
                message: format!("failed to load product categories: {e}"),
                details: None,
                request_id: None,
            }),
        )
    })?;

    let mut map = HashMap::new();
    for row in rows {
        map.insert(row.product_id, row.category);
    }
    Ok(map)
}

pub(crate) async fn fetch_existing_product_categories(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
) -> Result<HashSet<String>, (StatusCode, Json<ErrorResponse>)> {
    let rows: Vec<Option<String>> = sqlx::query_scalar(
        r#"
        select distinct category
        from products
        where tenant_id = $1
          and status <> 'DELETED'
          and category is not null
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_all(pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                code: "internal_error".to_string(),
                message: format!("failed to load categories: {e}"),
                details: None,
                request_id: None,
            }),
        )
    })?;

    Ok(rows
        .into_iter()
        .flatten()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect())
}

pub(crate) async fn fetch_existing_supplier_ids(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    supplier_ids: &[uuid::Uuid],
) -> Result<HashSet<uuid::Uuid>, (StatusCode, Json<ErrorResponse>)> {
    if supplier_ids.is_empty() {
        return Ok(HashSet::new());
    }
    let rows: Vec<uuid::Uuid> = sqlx::query_scalar(
        r#"
        select supplier_id
        from suppliers
        where tenant_id = $1
          and supplier_id = any($2)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(supplier_ids)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                code: "internal_error".to_string(),
                message: format!("failed to load suppliers: {e}"),
                details: None,
                request_id: None,
            }),
        )
    })?;
    Ok(rows.into_iter().collect())
}
