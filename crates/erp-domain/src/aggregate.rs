//! Report aggregates. A page total is not the full query, and a chart cannot invent its own numbers.

/// A page total is readable only beside the full query total.
/// Matching numbers do not make the page the full query. This is not a million-row timing run.
pub fn page_is_not_full(page_total: i64, full_total: i64, loaded: i64, matched: i64) -> Result<(), &'static str> {
    if page_total < 0 || full_total < 0 || loaded < 0 || matched < 0 {
        return Err("aggregate read is incomplete");
    }
    if loaded > matched {
        return Err("loaded rows exceed the full match");
    }
    if page_total == full_total && loaded < matched {
        return Err("page total cannot stand in for the full query");
    }
    Ok(())
}

/// Page and full query are the only scopes. A missing scope is refused.
pub fn admit_scope(scope: &str) -> Result<&'static str, &'static str> {
    match scope {
        "PAGE" => Ok("PAGE"),
        "FULL" => Ok("FULL"),
        _ => Err("aggregate scope is not registered"),
    }
}

/// An illegal or cyclic formula names the field. A script is not executed.
/// This is a field location, not a calculated million-row total.
pub fn locate_formula_field(field: &str, formula: &str) -> Result<&'static str, &'static str> {
    if field.trim().is_empty() {
        return Err("formula field is missing");
    }
    if formula.trim().is_empty() {
        return Err("formula is empty");
    }
    if formula.contains("script") || formula.contains(';') || formula.contains('{') {
        return Ok("ILLEGAL");
    }
    if formula.split('+').any(|part| part.trim() == field.trim()) {
        return Ok("CYCLIC");
    }
    Err("formula fault is not located")
}

/// A formula is arithmetic only. A script marker is refused.
pub fn admit_formula(formula: &str) -> Result<&'static str, &'static str> {
    if formula.trim().is_empty() {
        return Err("formula is empty");
    }
    if formula.contains("script") || formula.contains(';') || formula.contains('{') {
        return Err("formula cannot execute a script");
    }
    Ok("ARITHMETIC")
}

/// Collapse and expand do not change the stored total.
pub fn fold_keeps_total(before: i64, after: i64) -> Result<(), &'static str> {
    if before < 0 || before != after {
        return Err("folding changed the total");
    }
    Ok(())
}

/// A chart consumes the same query result. A different total is refused.
pub fn chart_matches_query(query_total: i64, chart_total: i64) -> Result<&'static str, &'static str> {
    if query_total < 0 || chart_total < 0 {
        return Err("chart total is incomplete");
    }
    if query_total == chart_total { Ok("SAME_QUERY") } else { Err("chart invented its own total") }
}

/// A grouped export must match the independent sum. A page sum cannot stand in for the full group.
pub fn group_matches_sql(exported: i64, sql_sum: i64, scope: &str) -> Result<(), &'static str> {
    admit_scope(scope)?;
    if exported != sql_sum {
        return Err("grouped export does not match the independent sum");
    }
    Ok(())
}
