//! Server grid query. A loaded page is not the full match, and a filter change blocks the old batch.

/// Business page sizes stay inside the host contract. Zero and values outside 100 to 2000 are refused.
pub fn admit_page_limit(limit: i32) -> Result<i32, &'static str> {
    if !matches!(limit, 100 | 200 | 300 | 500 | 800 | 1000 | 2000) {
        return Err("page limit is outside the host contract");
    }
    Ok(limit)
}

/// Equal sort keys append the stable row id. A sort without it can reorder a page.
pub fn sort_appends_row_id(sort_key: &str, row_id: &str) -> Result<&'static str, &'static str> {
    if sort_key.trim().is_empty() || row_id.trim().is_empty() {
        return Err("sort needs a key and a row id");
    }
    Ok("ROW_ID")
}

/// The returned total is exact, or it is named unknown. A loaded count cannot stand in for the match.
pub fn count_decision(loaded: i64, matched: i64, exact: bool) -> Result<&'static str, &'static str> {
    if loaded < 0 || matched < 0 || loaded > matched {
        return Err("count is inconsistent");
    }
    if exact { Ok("EXACT") } else { Ok("UNKNOWN") }
}

/// A page after the first 500 rows is still that filter. The loaded page is not the match.
pub fn page_after_first_block(offset: i64, limit: i32, loaded: i64, matched: i64) -> Result<(), &'static str> {
    if offset < 500 || limit <= 0 || loaded < 0 || matched <= offset {
        return Err("page after the first block is incomplete");
    }
    if loaded > i64::from(limit) || loaded > matched - offset {
        return Err("loaded page is larger than the remaining match");
    }
    Ok(())
}

/// A cross-page row stays selected when it is outside the loaded page.
pub fn cross_page_row_kept(kind: &str, row_id: &str, loaded: i64, matched: i64) -> Result<(), &'static str> {
    if kind != "CROSS_PAGE" || row_id.trim().is_empty() || loaded < 0 || matched <= loaded {
        return Err("cross-page selection is not outside the loaded page");
    }
    Ok(())
}

/// A batch may run only on the query that is still current. A page change or a new filter blocks it.
pub fn batch_on_query(current: &str, requested: &str) -> Result<&'static str, &'static str> {
    if current.trim().is_empty() || requested.trim().is_empty() {
        return Err("batch query is incomplete");
    }
    if current == requested { Ok("ALLOWED") } else { Ok("BLOCKED") }
}

/// A frozen match names the snapshot and the excluded rows. Loaded rows are not the frozen set.
pub fn frozen_match(snapshot: &str, excluded: i64, loaded: i64, matched: i64) -> Result<(), &'static str> {
    if snapshot.trim().is_empty() || excluded < 0 || loaded < 0 || matched < loaded {
        return Err("frozen match is incomplete");
    }
    if loaded == matched && excluded == 0 && snapshot == "LOADED" {
        return Err("loaded rows cannot stand in for the frozen match");
    }
    Ok(())
}
