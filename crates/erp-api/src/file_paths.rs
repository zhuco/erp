use super::*;

pub(crate) fn sku_import_base_dir() -> PathBuf {
    std::env::var("SKU_IMPORT_BASE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("imports/sku"))
}

pub(crate) fn sku_import_file_path(
    tenant_id: TenantId,
    file_id: uuid::Uuid,
    file_type: &str,
) -> PathBuf {
    let ext = if file_type.eq_ignore_ascii_case("CSV") {
        "csv"
    } else if file_type.eq_ignore_ascii_case("XLS") {
        "xls"
    } else {
        "xlsx"
    };
    let mut path = sku_import_base_dir();
    path.push(tenant_id.to_string());
    path.push(format!("sku-import-{file_id}.{ext}"));
    path
}

pub(crate) fn sku_import_template_path() -> PathBuf {
    std::env::var("SKU_IMPORT_TEMPLATE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("assets/sku_import_template.xlsx"))
}

pub(crate) fn sku_import_failure_file_path(tenant_id: TenantId, job_id: uuid::Uuid) -> PathBuf {
    let mut base = sku_import_failure_base_dir();
    base.push(tenant_id.to_string());
    base.push(format!("{job_id}.csv"));
    base
}

pub(crate) fn sku_import_failure_base_dir() -> PathBuf {
    std::env::var("SKU_IMPORT_FAILURE_BASE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("imports/sku_failures"))
}

pub(crate) fn export_job_file_path(tenant_id: TenantId, job_id: uuid::Uuid) -> PathBuf {
    let mut base = export_base_dir();
    base.push(tenant_id.to_string());
    base.push(format!("{job_id}.csv"));
    base
}

pub(crate) fn export_base_dir() -> PathBuf {
    std::env::var("EXPORT_BASE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("exports"))
}

pub(crate) fn after_sales_import_file_path(
    tenant_id: TenantId,
    file_id: uuid::Uuid,
    file_type: &str,
) -> PathBuf {
    let ext = if file_type.eq_ignore_ascii_case("CSV") {
        "csv"
    } else if file_type.eq_ignore_ascii_case("XLS") {
        "xls"
    } else {
        "xlsx"
    };
    let mut base = after_sales_import_base_dir();
    base.push(tenant_id.to_string());
    base.push(format!("{file_id}.{ext}"));
    base
}

pub(crate) fn after_sales_import_base_dir() -> PathBuf {
    std::env::var("AFTER_SALES_IMPORT_BASE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("imports/after_sales"))
}
