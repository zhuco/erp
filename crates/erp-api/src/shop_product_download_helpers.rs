use super::*;

pub(crate) fn build_shop_product_download_items_from_adapter(
    products: &[erp_adapter::PlatformProduct],
    default_product_type: Option<String>,
    default_status: Option<String>,
) -> Vec<ShopProductDownloadItem> {
    let fallback_type =
        normalize_optional_string(default_product_type).unwrap_or_else(|| "STANDARD".to_string());
    let fallback_status =
        normalize_optional_string(default_status).unwrap_or_else(|| "ACTIVE".to_string());
    products
        .iter()
        .map(|product| ShopProductDownloadItem {
            platform_product_id: product.platform_product_id.clone(),
            merchant_code: None,
            title: product.name.clone(),
            product_type: Some(fallback_type.clone()),
            status: Some(fallback_status.clone()),
            updated_at: None,
            raw_json: Some(product.raw.clone()),
            skus: product
                .skus
                .iter()
                .map(|sku| ShopProductDownloadSku {
                    platform_sku_id: sku.platform_sku_id.clone(),
                    merchant_sku_code: normalize_optional_string(sku.sku_code.clone()),
                    title: sku.name.clone(),
                    spec_text: extract_spec_text_from_platform_spec(sku.spec.as_ref()),
                    price_cent: None,
                    stock_qty: None,
                    status: Some(fallback_status.clone()),
                    barcode: sku.barcode.clone(),
                    image_url: sku.image_urls.first().cloned(),
                    updated_at: None,
                    raw_json: Some(sku.raw.clone()),
                })
                .collect(),
        })
        .collect()
}

pub(crate) fn extract_spec_text_from_platform_spec(spec: Option<&Value>) -> Option<String> {
    let spec = spec?;
    match spec {
        Value::Object(map) => map
            .get("spec_text")
            .or_else(|| map.get("spec"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        Value::String(value) => Some(value.clone()),
        _ => None,
    }
}
