//! Forecast and subscription rules. A forecast is not a sales, stock, or production fact.

/// A subscription is delivered only to an active recipient inside the saved scope.
pub fn admit_report_subscription(owner: &str, recipient: &str, scope: &str, active: bool) -> Result<(), &'static str> {
    if owner.trim().is_empty() || recipient.trim().is_empty() || scope.trim().is_empty() {
        return Err("subscription identity is incomplete");
    }
    if !active {
        return Err("a disabled account cannot receive a report");
    }
    if recipient != owner {
        return Err("recipient is outside the saved scope");
    }
    Ok(())
}

/// Training ends before the holdout starts. The model version and cutoff stay with the forecast.
pub fn admit_forecast_split(
    model_version: &str,
    cutoff: &str,
    holdout_from: &str,
    holdout_to: &str,
) -> Result<(), &'static str> {
    if model_version.trim().is_empty() || cutoff.trim().is_empty() || holdout_from.trim().is_empty() || holdout_to.trim().is_empty() {
        return Err("forecast split is incomplete");
    }
    if cutoff >= holdout_from || holdout_from >= holdout_to {
        return Err("forecast training overlaps the holdout");
    }
    Ok(())
}

/// Error is absolute deviation. A forecast without a naive baseline stays exploratory.
pub fn forecast_error(predicted: i64, actual: i64, naive: i64) -> Result<(i64, &'static str), &'static str> {
    if predicted < 0 || actual < 0 || naive < 0 {
        return Err("forecast quantities cannot be negative");
    }
    let error = (predicted - actual).abs();
    let baseline = (naive - actual).abs();
    let status = if error < baseline { "COMPARED" } else { "EXPLORATORY" };
    Ok((error, status))
}
