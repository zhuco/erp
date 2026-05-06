use super::*;

#[allow(clippy::result_large_err)]
pub(crate) fn parse_sku_import_rows(
    content: &[u8],
    file_type: &str,
) -> Result<Vec<SkuImportRow>, Response> {
    if file_type.eq_ignore_ascii_case("CSV") {
        return parse_sku_import_csv(content);
    }
    parse_sku_import_excel(content, file_type)
}

#[allow(clippy::result_large_err)]
pub(crate) fn parse_sku_import_csv(content: &[u8]) -> Result<Vec<SkuImportRow>, Response> {
    let mut reader = ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(Cursor::new(content));
    let headers = reader
        .headers()
        .map_err(|_| bad_request("invalid csv headers").into_response())?
        .iter()
        .map(normalize_import_header)
        .collect::<Vec<_>>();
    let indexes = build_sku_import_header_index(&headers)?;

    let mut rows = Vec::new();
    for (index, record) in reader.records().enumerate() {
        let record = record.map_err(|_| bad_request("invalid csv row").into_response())?;
        let row = parse_sku_import_row(
            index as i64 + 2,
            |idx| {
                idx.and_then(|i| record.get(i))
                    .map(|value| value.to_string())
            },
            &indexes,
        );
        rows.push(row);
    }
    Ok(rows)
}

#[allow(clippy::result_large_err)]
pub(crate) fn parse_sku_import_excel(
    content: &[u8],
    file_type: &str,
) -> Result<Vec<SkuImportRow>, Response> {
    let temp_path = write_temp_import_file(content, file_type)?;
    let mut workbook = open_workbook_auto(&temp_path)
        .map_err(|_| bad_request("invalid excel file").into_response())?;
    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| bad_request("missing sheet").into_response())?;
    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|_| bad_request("invalid sheet").into_response())?;

    let mut rows_iter = range.rows();
    let header_row = rows_iter
        .next()
        .ok_or_else(|| bad_request("empty sheet").into_response())?;
    let headers = header_row
        .iter()
        .map(|cell| normalize_import_header(&cell.to_string()))
        .collect::<Vec<_>>();
    let indexes = build_sku_import_header_index(&headers)?;

    let mut rows = Vec::new();
    for (index, row) in rows_iter.enumerate() {
        let values = row.iter().map(cell_string).collect::<Vec<_>>();
        let parsed = parse_sku_import_row(
            index as i64 + 2,
            |idx| idx.and_then(|i| values.get(i)).cloned(),
            &indexes,
        );
        rows.push(parsed);
    }

    let _ = fs::remove_file(&temp_path);
    Ok(rows)
}

#[allow(clippy::result_large_err)]
pub(crate) fn write_temp_import_file(content: &[u8], file_type: &str) -> Result<PathBuf, Response> {
    let mut path = std::env::temp_dir();
    let ext = if file_type.eq_ignore_ascii_case("CSV") {
        "csv"
    } else if file_type.eq_ignore_ascii_case("XLS") {
        "xls"
    } else {
        "xlsx"
    };
    path.push(format!("sku-import-{}.{}", uuid::Uuid::new_v4(), ext));
    fs::write(&path, content).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to write temp file",
        )
            .into_response()
    })?;
    Ok(path)
}

fn parse_sku_import_row<F: Fn(Option<usize>) -> Option<String>>(
    row_number: i64,
    get: F,
    indexes: &SkuImportHeaderIndex,
) -> SkuImportRow {
    let product_id = get(indexes.product_id).and_then(|value| optional_string(&value));
    let product_name = get(indexes.product_name).and_then(|value| optional_string(&value));
    let sku_code = get(indexes.sku_code).unwrap_or_default();
    let sku_code = sku_code.trim().to_string();
    let barcode = get(indexes.barcode).and_then(|value| optional_string(&value));
    let aux_barcode = get(indexes.aux_barcode).and_then(|value| optional_string(&value));
    let name = get(indexes.name).and_then(|value| optional_string(&value));
    let spec = get(indexes.spec).and_then(|value| optional_string(&value));
    let unit = get(indexes.unit).and_then(|value| optional_string(&value));
    let category_id = get(indexes.category_id).and_then(|value| optional_string(&value));
    let category_name = get(indexes.category_name).and_then(|value| optional_string(&value));
    let sku_kind = get(indexes.sku_kind).and_then(|value| optional_string(&value));
    let tags_raw = get(indexes.tags).unwrap_or_default();
    let tags = normalize_string_list(split_import_list(tags_raw.as_str()));
    let image_urls_raw = get(indexes.image_urls).unwrap_or_default();
    let image_urls = normalize_string_list(split_import_list(image_urls_raw.as_str()));
    let cost_cent = get(indexes.cost_cent).and_then(|value| optional_string(&value));
    let supplier_ids_raw = get(indexes.supplier_ids).unwrap_or_default();
    let supplier_ids = normalize_string_list(split_import_list(supplier_ids_raw.as_str()));
    let weight_gram = get(indexes.weight_gram).and_then(|value| optional_string(&value));
    let length_mm = get(indexes.length_mm).and_then(|value| optional_string(&value));
    let width_mm = get(indexes.width_mm).and_then(|value| optional_string(&value));
    let height_mm = get(indexes.height_mm).and_then(|value| optional_string(&value));
    let volume_cm3 = get(indexes.volume_cm3).and_then(|value| optional_string(&value));

    SkuImportRow {
        row: row_number,
        product_id,
        product_name,
        sku_code,
        barcode,
        aux_barcode,
        name,
        spec,
        unit,
        category_id,
        category_name,
        sku_kind,
        tags,
        image_urls,
        cost_cent,
        supplier_ids,
        weight_gram,
        length_mm,
        width_mm,
        height_mm,
        volume_cm3,
    }
}

pub(crate) fn normalize_import_header(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace([' ', '-', '/', '.', '(', ')', '（', '）'], "_")
        .replace("__", "_")
        .trim_matches('_')
        .to_string()
}

#[allow(clippy::result_large_err)]
pub(crate) fn build_sku_import_header_index(
    headers: &[String],
) -> Result<SkuImportHeaderIndex, Response> {
    let mut index = SkuImportHeaderIndex::default();
    for (idx, header) in headers.iter().enumerate() {
        match header.as_str() {
            "product_id" | "productid" | "货品编号" | "鍟嗗搧编号" | "浜у搧编号" => {
                index.product_id = Some(idx)
            }
            "product_name" | "product" | "productname" | "货品鍚嶇О" | "鍟嗗搧鍚嶇О"
            | "浜у搧鍚嶇О" => index.product_name = Some(idx),
            "sku_code" | "sku" | "skucode" | "规格编号" | "规格编码" | "sku编号" | "sku缂栫爜" => {
                index.sku_code = Some(idx)
            }
            "barcode" | "bar_code" | "鏉＄爜" => index.barcode = Some(idx),
            "aux_barcode" | "assistant_barcode" | "辅助条码" | "辅助码" => {
                index.aux_barcode = Some(idx)
            }
            "name" | "sku_name" | "sku鍚嶇О" | "瑙勬牸鍚嶇О" => index.name = Some(idx),
            "spec" | "spec_info" | "规格信息" | "规格" => index.spec = Some(idx),
            "unit" | "鍗曚綅" => index.unit = Some(idx),
            "category_id" | "categoryid" | "分类编号" | "鍒嗙被id" | "分类编码" => {
                index.category_id = Some(idx)
            }
            "category_name" | "category" | "鍒嗙被鍚嶇О" | "鍒嗙被" => {
                index.category_name = Some(idx)
            }
            "sku_kind" | "kind" | "type" | "瑙勬牸绫诲瀷" | "sku绫诲瀷" => {
                index.sku_kind = Some(idx)
            }
            "tags" | "tag" | "标签" => index.tags = Some(idx),
            "image_urls" | "images" | "image_url" | "鍥剧墖" | "鍥剧墖閾炬帴" => {
                index.image_urls = Some(idx)
            }
            "cost_cent" | "cost" | "鎴愭湰" => index.cost_cent = Some(idx),
            "supplier_ids" | "suppliers" | "supplier_id" | "供应商" | "供应商id" => {
                index.supplier_ids = Some(idx)
            }
            "weight_gram" | "weight" | "weight_g" | "閲嶉噺" | "閲嶉噺g" | "閲嶉噺_g" => {
                index.weight_gram = Some(idx)
            }
            "length_mm" | "length" | "闀垮害" | "闀垮害_mm" => index.length_mm = Some(idx),
            "width_mm" | "width" | "瀹藉害" | "瀹藉害_mm" => index.width_mm = Some(idx),
            "height_mm" | "height" | "楂樺害" | "楂樺害_mm" => index.height_mm = Some(idx),
            "volume_cm3" | "volume" | "浣撶Н" | "浣撶Н_cm3" => index.volume_cm3 = Some(idx),
            _ => {}
        }
    }
    if index.sku_code.is_none() {
        return Err(bad_request("missing sku_code column").into_response());
    }
    if index.product_id.is_none() && index.product_name.is_none() {
        return Err(bad_request("missing product_id or product_name column").into_response());
    }
    if index.unit.is_none() {
        return Err(bad_request("missing unit column").into_response());
    }
    if index.category_id.is_none() && index.category_name.is_none() {
        return Err(bad_request("missing category column").into_response());
    }
    Ok(index)
}

pub(crate) fn split_import_list(raw: &str) -> Vec<String> {
    raw.split([',', ';', '|'])
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect()
}

pub(crate) fn optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(crate) fn cell_string(cell: &Data) -> String {
    match cell {
        Data::String(value) => value.trim().to_string(),
        Data::Float(value) => {
            if value.fract() == 0.0 {
                format!("{:.0}", value)
            } else {
                value.to_string()
            }
        }
        Data::Int(value) => value.to_string(),
        Data::Bool(value) => value.to_string(),
        _ => cell.to_string(),
    }
}

pub(crate) fn push_import_issue(
    errors: &mut Vec<SkuImportIssue>,
    invalid_rows: &mut HashSet<i64>,
    row: i64,
    field: Option<&str>,
    message: &str,
) {
    errors.push(SkuImportIssue {
        row,
        field: field.map(|value| value.to_string()),
        message: message.to_string(),
    });
    invalid_rows.insert(row);
}

pub(crate) fn parse_optional_i64_field(
    raw: Option<&str>,
    field: &str,
    row: i64,
    errors: &mut Vec<SkuImportIssue>,
    invalid_rows: &mut HashSet<i64>,
) -> Option<i64> {
    let value = raw.map(str::trim).filter(|v| !v.is_empty())?;
    match value.parse::<i64>() {
        Ok(v) => Some(v),
        Err(_) => {
            push_import_issue(errors, invalid_rows, row, Some(field), "invalid number");
            None
        }
    }
}

pub(crate) fn parse_optional_i32_field(
    raw: Option<&str>,
    field: &str,
    row: i64,
    errors: &mut Vec<SkuImportIssue>,
    invalid_rows: &mut HashSet<i64>,
) -> Option<i32> {
    let value = raw.map(str::trim).filter(|v| !v.is_empty())?;
    match value.parse::<i32>() {
        Ok(v) => Some(v),
        Err(_) => {
            push_import_issue(errors, invalid_rows, row, Some(field), "invalid number");
            None
        }
    }
}
