use super::*;

pub(crate) fn parse_role_ids(
    role_ids: &[String],
) -> Result<Vec<RoleId>, (StatusCode, Json<ErrorResponse>)> {
    let mut ids = Vec::with_capacity(role_ids.len());
    for raw in role_ids {
        match RoleId::from_str(raw) {
            Ok(id) => ids.push(id),
            Err(_) => return Err(bad_request("bad role_id")),
        }
    }
    Ok(ids)
}

pub(crate) fn parse_shop_ids(
    shop_ids: &[String],
) -> Result<Vec<ShopId>, (StatusCode, Json<ErrorResponse>)> {
    let mut ids = Vec::with_capacity(shop_ids.len());
    for raw in shop_ids {
        match ShopId::from_str(raw) {
            Ok(id) => ids.push(id),
            Err(_) => return Err(bad_request("bad shop_id")),
        }
    }
    Ok(ids)
}

pub(crate) fn parse_uuid_list(
    ids: &[String],
) -> Result<Vec<uuid::Uuid>, (StatusCode, Json<ErrorResponse>)> {
    let mut values = Vec::with_capacity(ids.len());
    for raw in ids {
        match uuid::Uuid::parse_str(raw) {
            Ok(id) => values.push(id),
            Err(_) => return Err(bad_request("bad uuid")),
        }
    }
    Ok(values)
}

pub(crate) fn parse_uuid_list_optional(
    ids: Option<&[String]>,
) -> Result<Option<Vec<uuid::Uuid>>, (StatusCode, Json<ErrorResponse>)> {
    let Some(ids) = ids else {
        return Ok(None);
    };
    let parsed = parse_uuid_list(ids)?;
    if parsed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(parsed))
    }
}

pub(crate) fn parse_shop_ids_csv(
    raw: Option<&str>,
) -> Result<Option<Vec<ShopId>>, (StatusCode, Json<ErrorResponse>)> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let ids: Vec<String> = raw
        .split(',')
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect();
    if ids.is_empty() {
        return Ok(None);
    }
    parse_shop_ids(&ids).map(Some)
}

pub(crate) fn parse_shop_ids_optional(
    shop_ids: Option<&[String]>,
) -> Result<Option<Vec<ShopId>>, (StatusCode, Json<ErrorResponse>)> {
    let Some(shop_ids) = shop_ids else {
        return Ok(None);
    };
    let parsed = parse_shop_ids(shop_ids)?;
    if parsed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(parsed))
    }
}

pub(crate) fn parse_platform_optional(
    raw: Option<&str>,
) -> Result<Option<Platform>, (StatusCode, Json<ErrorResponse>)> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    match Platform::from_str(trimmed) {
        Ok(v) => Ok(Some(v)),
        Err(_) => Err(bad_request("bad platform")),
    }
}

pub(crate) fn normalize_optional_string(input: Option<String>) -> Option<String> {
    input.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub(crate) fn parse_datetime_optional(
    raw: Option<&str>,
    field: &str,
) -> Result<Option<DateTime<Utc>>, (StatusCode, Json<ErrorResponse>)> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if let Ok(value) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(Some(value.with_timezone(&Utc)));
    }
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        let dt = date.and_hms_opt(0, 0, 0).unwrap();
        return Ok(Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)));
    }
    Err(bad_request(&format!("bad {field}")))
}

pub(crate) fn parse_date_optional(
    raw: Option<&str>,
    field: &str,
) -> Result<Option<NaiveDate>, (StatusCode, Json<ErrorResponse>)> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return Ok(Some(date));
    }
    if let Ok(value) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(Some(value.date_naive()));
    }
    Err(bad_request(&format!("bad {field}")))
}

pub(crate) fn parse_datetime_best_effort(raw: Option<&str>) -> Option<DateTime<Utc>> {
    let raw = raw?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(value) = DateTime::parse_from_rfc3339(trimmed) {
        return Some(value.with_timezone(&Utc));
    }
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        let dt = date.and_hms_opt(0, 0, 0).unwrap();
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    }
    None
}

pub(crate) fn dedup_uuid_list(mut items: Vec<uuid::Uuid>) -> Vec<uuid::Uuid> {
    items.sort();
    items.dedup();
    items
}
