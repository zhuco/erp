//! Shared write receipt. A business effect is complete only after the
//! receipt says so. Repeating the same command returns the original effect.

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandStatus {
    Accepted,
    Running,
    Succeeded,
    Failed,
    Reconciling,
}

impl CommandStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "ACCEPTED",
            Self::Running => "RUNNING",
            Self::Succeeded => "SUCCEEDED",
            Self::Failed => "FAILED",
            Self::Reconciling => "RECONCILING",
        }
    }

    pub fn is_complete(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandReceipt {
    pub request_id: String,
    pub payload_hash: String,
    pub status: CommandStatus,
    pub effect_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ReceiptError {
    #[error("request id is empty")]
    EmptyRequest,
    #[error("same request id carries a different payload")]
    PayloadConflict,
}

/// Same request and same payload returns the stored effect. A different payload
/// is a conflict and must not create a second effect.
pub fn admit_retry(
    stored: Option<&CommandReceipt>,
    request_id: &str,
    payload_hash: &str,
) -> Result<Option<CommandReceipt>, ReceiptError> {
    let request_id = request_id.trim();
    if request_id.is_empty() {
        return Err(ReceiptError::EmptyRequest);
    }
    let Some(stored) = stored else {
        return Ok(None);
    };
    if stored.request_id != request_id {
        return Err(ReceiptError::PayloadConflict);
    }
    if stored.payload_hash != payload_hash {
        return Err(ReceiptError::PayloadConflict);
    }
    Ok(Some(stored.clone()))
}

/// Shared inbound payload identity. Quantity stays a legacy whole base unit.
pub fn inbound_payload(warehouse_id: &str, sku_id: &str, quantity: i64, reason: &str) -> String {
    format!(
        "{}:{}:{}:{}",
        warehouse_id.trim(),
        sku_id.trim(),
        quantity,
        reason.trim()
    )
}

/// One command posts once. A matching retry returns the stored ledger identity.
/// A different payload is a conflict and must not add stock again.
pub fn admit_inbound_retry(
    stored: Option<&CommandReceipt>,
    request_id: &str,
    warehouse_id: &str,
    sku_id: &str,
    quantity: i64,
    reason: &str,
) -> Result<Option<CommandReceipt>, ReceiptError> {
    admit_retry(
        stored,
        request_id,
        &inbound_payload(warehouse_id, sku_id, quantity, reason),
    )
}

/// Client timeout is unknown, not a server failure. A stored success stays stored.
pub fn client_unknown_is_not_failure(server_status: Option<CommandStatus>) -> CommandStatus {
    match server_status {
        Some(status) => status,
        None => CommandStatus::Reconciling,
    }
}

/// Lost-response lookup only observes the original server status.
/// Neither a client timeout nor its disappearance authorizes a new business effect.
pub fn lost_response_retry_decision(
    stored_status: CommandStatus,
    _client_unknown: bool,
) -> Result<&'static str, ReceiptError> {
    if stored_status.is_complete() {
        Ok("KEEP")
    } else {
        Ok("LOOKUP")
    }
}

/// A revoked session or a lost response must not erase a committed effect.
pub fn committed_effect_survives_session_loss(status: CommandStatus) -> bool {
    matches!(status, CommandStatus::Succeeded | CommandStatus::Failed)
}

pub fn command_status(name: &str) -> Result<CommandStatus, ReceiptError> {
    match name {
        "ACCEPTED" => Ok(CommandStatus::Accepted),
        "RUNNING" => Ok(CommandStatus::Running),
        "SUCCEEDED" => Ok(CommandStatus::Succeeded),
        "FAILED" => Ok(CommandStatus::Failed),
        "RECONCILING" => Ok(CommandStatus::Reconciling),
        _ => Err(ReceiptError::PayloadConflict),
    }
}

/// Zero affected rows is a version conflict. The caller keeps the uncommitted input.
pub fn version_conflict(affected_rows: u64) -> bool {
    affected_rows == 0
}

/// A completed read may fill the page only when its generation is still current.
pub fn stale_read_may_apply(
    read_generation: i64,
    current_generation: i64,
    cancelled: bool,
) -> bool {
    !cancelled && read_generation == current_generation && read_generation > 0
}

/// A newer read cancels only the same scope. An accepted write stays until its own receipt returns.
pub fn read_cancels_same_scope(
    stored_scope: &str,
    requested_scope: &str,
    stored_generation: i64,
    requested_generation: i64,
    write_accepted: bool,
) -> bool {
    stored_scope == requested_scope && stored_generation < requested_generation && !write_accepted
}

pub const TARGET_RPO_SECONDS: i64 = 15 * 60;
pub const TARGET_RTO_SECONDS: i64 = 2 * 60 * 60;

/// A backup file name is not a recovery. The database cutoff and attachment cutoff stay separate.
pub fn admit_recovery_checkpoint(
    database_cutoff: &str,
    attachment_cutoff: &str,
    backup_digest: &str,
) -> Result<(), ReceiptError> {
    if database_cutoff.trim().is_empty()
        || attachment_cutoff.trim().is_empty()
        || backup_digest.trim().is_empty()
    {
        return Err(ReceiptError::EmptyRequest);
    }
    if database_cutoff == attachment_cutoff {
        return Err(ReceiptError::PayloadConflict);
    }
    Ok(())
}

/// A shared-database rewind is refused. One tenant is compared in an isolated target first.
pub fn admit_recovery_target(
    shared_database: bool,
    isolated_target: bool,
    tenant_scoped: bool,
) -> Result<(), ReceiptError> {
    if shared_database || !isolated_target || !tenant_scoped {
        return Err(ReceiptError::PayloadConflict);
    }
    Ok(())
}

/// A restored pending task must not post the stock effect a second time.
pub fn restored_task_does_not_repost(already_posted: bool) -> &'static str {
    if already_posted { "KEEP" } else { "POST" }
}

/// Missing archive, damaged backup, or missing attachment blocks apply. A pending task is not replayed.
pub fn recovery_compare(
    archive_present: bool,
    backup_intact: bool,
    attachment_present: bool,
    pending_tasks: i64,
) -> Result<&'static str, ReceiptError> {
    if !archive_present {
        return Ok("ARCHIVE_MISSING");
    }
    if !backup_intact {
        return Ok("BACKUP_DAMAGED");
    }
    if !attachment_present {
        return Ok("ATTACHMENT_MISSING");
    }
    if pending_tasks < 0 {
        return Err(ReceiptError::PayloadConflict);
    }
    Ok("COMPARED")
}

pub const RELEASE_EXPAND: &str = "EXPAND";
pub const RELEASE_SWITCH: &str = "SWITCH";
pub const RELEASE_CONTRACT: &str = "CONTRACT";

/// Expand, then switch, then contract. A skipped phase is refused.
pub fn admit_release_phase(stored: Option<&str>, requested: &str) -> Result<(), ReceiptError> {
    let allowed = match stored {
        None => RELEASE_EXPAND,
        Some(RELEASE_EXPAND) => RELEASE_SWITCH,
        Some(RELEASE_SWITCH) => RELEASE_CONTRACT,
        _ => return Err(ReceiptError::PayloadConflict),
    };
    if requested == allowed {
        Ok(())
    } else {
        Err(ReceiptError::PayloadConflict)
    }
}

/// An artifact rollback copies the fact. It must not replace the committed amount or quantity.
pub fn rollback_keeps_fact(
    before_amount: i64,
    before_qty: i64,
    after_amount: i64,
    after_qty: i64,
) -> Result<(), ReceiptError> {
    if before_amount == after_amount && before_qty == after_qty {
        Ok(())
    } else {
        Err(ReceiptError::PayloadConflict)
    }
}

/// Current and previous clients may write while the window is open. An older client keeps its draft.
pub fn client_submit_decision(
    client_release: i32,
    current_release: i32,
    previous_release: i32,
    window_open: bool,
) -> Result<&'static str, ReceiptError> {
    if current_release < 1
        || previous_release < 1
        || previous_release >= current_release
        || client_release < 1
    {
        return Err(ReceiptError::PayloadConflict);
    }
    if client_release == current_release || (window_open && client_release == previous_release) {
        return Ok("WRITE");
    }
    if client_release < current_release {
        return Ok("DRAFT_KEPT");
    }
    Err(ReceiptError::PayloadConflict)
}

/// A legacy whole-unit integer stays one base unit at the shared six-digit scale.
pub fn legacy_qty_micros(units: i64) -> Result<i64, ReceiptError> {
    crate::quantity::ScaledQty::from_legacy_whole_units(units)
        .map(|qty| qty.as_micros())
        .map_err(|_| ReceiptError::PayloadConflict)
}

/// A public origin is an exact HTTPS name. A wildcard or a development HTTP origin is not public.
pub fn admit_public_origin(origin: &str, allowed: &[String]) -> Result<(), ReceiptError> {
    let origin = origin.trim();
    if origin.is_empty() || origin == "*" || !origin.starts_with("https://") || origin.contains('*')
    {
        return Err(ReceiptError::PayloadConflict);
    }
    if !allowed.iter().any(|item| item == origin) {
        return Err(ReceiptError::PayloadConflict);
    }
    Ok(())
}

/// Cookie mode needs HttpOnly, Secure, SameSite, and a CSRF token. Bearer mode does not use a cookie.
pub fn admit_browser_session(
    mode: &str,
    http_only: bool,
    secure: bool,
    same_site: &str,
    csrf_token: &str,
) -> Result<(), ReceiptError> {
    match mode {
        "BEARER" => Ok(()),
        "COOKIE"
            if http_only
                && secure
                && matches!(same_site, "Strict" | "Lax")
                && !csrf_token.trim().is_empty() =>
        {
            Ok(())
        }
        "COOKIE" => Err(ReceiptError::PayloadConflict),
        _ => Err(ReceiptError::PayloadConflict),
    }
}

pub fn csrf_matches(mode: &str, stored: &str, presented: &str) -> bool {
    mode != "COOKIE" || (!stored.is_empty() && stored == presented)
}

/// A client package must not carry a database or object-storage master key.
pub fn client_material_is_safe(material: &str) -> bool {
    let lower = material.to_ascii_lowercase();
    !lower.contains("database_url")
        && !lower.contains("master_key")
        && !lower.contains("secret_access_key")
}

/// Diagnostic text is stored only after secrets are removed.
pub fn redact_diagnostic(raw: &str) -> Result<String, ReceiptError> {
    let mut kept = Vec::new();
    for part in raw.split_whitespace() {
        let lower = part.to_ascii_lowercase();
        if lower.contains("password")
            || lower.contains("token")
            || lower.contains("secret")
            || lower.contains("database_url")
        {
            continue;
        }
        kept.push(part);
    }
    if kept.is_empty() {
        return Err(ReceiptError::EmptyRequest);
    }
    Ok(kept.join(" "))
}

/// A download link is valid only for the same tenant, actor, and stored object, before expiry and revocation.
pub fn download_link_allowed(
    tenant_matches: bool,
    actor_matches: bool,
    object_matches: bool,
    expired: bool,
) -> Result<(), ReceiptError> {
    download_link_still_open(tenant_matches, actor_matches, object_matches, expired, false)
}

/// A revoked link is refused even when it has not expired.
pub fn download_link_still_open(
    tenant_matches: bool,
    actor_matches: bool,
    object_matches: bool,
    expired: bool,
    revoked: bool,
) -> Result<(), ReceiptError> {
    if tenant_matches && actor_matches && object_matches && !expired && !revoked {
        Ok(())
    } else {
        Err(ReceiptError::PayloadConflict)
    }
}

pub fn credential_epoch_advanced(old_epoch: i32, new_epoch: i32) -> bool {
    new_epoch == old_epoch.saturating_add(1) && new_epoch > old_epoch
}

pub const CLIENT_ARTIFACT_VERSION: &str = "0.1.0";
pub const CLIENT_SERVER_RANGE: &str = "erp-api-v1";

/// A named package is banned, missing a license, or acceptable. The caller does not choose the result.
/// An exemption needs a name, a scope, and a date that is still open on the scan day.
pub fn classify_license_scan(
    package_name: &str,
    license_name: &str,
    exemption_scope: &str,
    exemption_expires: &str,
    scan_day: &str,
) -> Result<&'static str, ReceiptError> {
    let package = package_name.trim();
    if package.is_empty() || scan_day.trim().is_empty() {
        return Err(ReceiptError::EmptyRequest);
    }
    let banned = package.to_ascii_lowercase().contains("ag-grid")
        || package.to_ascii_lowercase().contains("ag grid");
    let exempt = !exemption_scope.trim().is_empty()
        && !exemption_expires.trim().is_empty()
        && exemption_expires.as_bytes() >= scan_day.as_bytes();
    if banned && !exempt {
        return Ok("BANNED");
    }
    let license = license_name.trim();
    let allowed = matches!(
        license,
        "MIT"
            | "Apache-2.0"
            | "BSD-3-Clause"
            | "ISC"
            | "MPL-2.0"
            | "Unicode-3.0"
            | "OFL-1.1"
            | "Proprietary"
    );
    if !allowed {
        return Ok("MISSING");
    }
    Ok("OK")
}

/// A release gate passes only when this artifact has no banned package, no missing license, and a matching SHA.
pub fn admit_release_gate(
    artifact_sha: &str,
    bound_sha: &str,
    banned_count: i64,
    missing_license_count: i64,
    scanned_count: i64,
) -> Result<(), ReceiptError> {
    if artifact_sha.trim().is_empty()
        || artifact_sha != bound_sha
        || banned_count != 0
        || missing_license_count != 0
        || scanned_count < 1
    {
        return Err(ReceiptError::PayloadConflict);
    }
    Ok(())
}

/// The gate must name the same artifact, migration list, previous client, and gray switch as the release.
/// A closed window cannot stay gray. The minimum client is the previous release while the window is open.
#[derive(Debug, Clone, Copy)]
pub struct ReleaseGateInput<'a> {
    pub artifact_sha: &'a str,
    pub bound_sha: &'a str,
    pub migration_list: &'a str,
    pub recorded_list: &'a str,
    pub minimum_client: i32,
    pub previous_release: i32,
    pub gray_switch: bool,
    pub window_open: bool,
}

pub fn admit_release_gate_record(input: ReleaseGateInput<'_>) -> Result<(), ReceiptError> {
    let ReleaseGateInput {
        artifact_sha,
        bound_sha,
        migration_list,
        recorded_list,
        minimum_client,
        previous_release,
        gray_switch,
        window_open,
    } = input;
    if artifact_sha.trim().is_empty()
        || artifact_sha != bound_sha
        || migration_list.trim().is_empty()
        || migration_list != recorded_list
        || minimum_client != previous_release
        || (gray_switch && !window_open)
    {
        return Err(ReceiptError::PayloadConflict);
    }
    Ok(())
}

/// Desktop and web share one version and server range. Each names its own start command.
pub fn client_launch_note(platform: &str) -> Result<&'static str, ReceiptError> {
    match platform {
        "WINDOWS" => Ok("erp-grid-lab.exe --erp"),
        "WASM" => Ok("index.html?erp"),
        _ => Err(ReceiptError::PayloadConflict),
    }
}

/// A launch note must name the shared version, the server range, and that platform's start.
pub fn admit_client_launch(
    platform: &str,
    version: &str,
    server_range: &str,
    note: &str,
) -> Result<(), ReceiptError> {
    let start = client_launch_note(platform)?;
    if version != CLIENT_ARTIFACT_VERSION || server_range != CLIENT_SERVER_RANGE || note != start {
        return Err(ReceiptError::PayloadConflict);
    }
    Ok(())
}

/// A releasable client is the Windows desktop binary or the WASM web package. A module fixture is neither.
pub fn admit_client_artifact(
    platform: &str,
    artifact: &str,
    entry: &str,
) -> Result<(), ReceiptError> {
    let ok = match platform {
        "WINDOWS" => artifact == "erp-grid-lab.exe" && entry == "--erp",
        "WASM" => artifact == "erp_ui.wasm" && entry == "data-erp-app",
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(ReceiptError::PayloadConflict)
    }
}

/// Each platform owns its own adapter. A missing adapter is not implied by the other platform.
pub fn admit_platform_adapter(platform: &str, adapter: &str) -> Result<(), ReceiptError> {
    if !matches!(platform, "WINDOWS" | "WASM") {
        return Err(ReceiptError::PayloadConflict);
    }
    if matches!(
        adapter,
        "NETWORK" | "AUTH" | "DOWNLOAD" | "CLIPBOARD" | "PERSIST" | "PRINT" | "LIFECYCLE"
    ) {
        Ok(())
    } else {
        Err(ReceiptError::PayloadConflict)
    }
}

/// A drill names an owner and a stop action. An unowned automatic script is refused.
pub fn admit_operations_drill(
    kind: &str,
    owner: &str,
    stop_action: &str,
) -> Result<(), ReceiptError> {
    if !matches!(
        kind,
        "BACKUP_CHECK"
            | "RESTORE_DRILL"
            | "STORAGE_CHANGE"
            | "DATABASE_CHANGE"
            | "CAPACITY"
            | "INSPECTION"
    ) {
        return Err(ReceiptError::PayloadConflict);
    }
    if owner.trim().is_empty() || stop_action.trim().is_empty() {
        return Err(ReceiptError::PayloadConflict);
    }
    Ok(())
}

/// A monthly backup check, a quarterly restore, or a drill after a storage or database change is due when the previous date is older than the cadence.
pub fn drill_is_due(kind: &str, previous: &str, today: &str) -> Result<bool, ReceiptError> {
    let cadence = match kind {
        "BACKUP_CHECK" => 31,
        "RESTORE_DRILL" => 92,
        "STORAGE_CHANGE" | "DATABASE_CHANGE" => 1,
        _ => return Err(ReceiptError::PayloadConflict),
    };
    let previous = date_ordinal(previous)?;
    let today = date_ordinal(today)?;
    if today < previous {
        return Err(ReceiptError::PayloadConflict);
    }
    Ok(today - previous >= cadence)
}

fn date_ordinal(value: &str) -> Result<i32, ReceiptError> {
    let parts: Vec<_> = value.split('-').collect();
    if parts.len() != 3 {
        return Err(ReceiptError::PayloadConflict);
    }
    let year: i32 = parts[0]
        .parse()
        .map_err(|_| ReceiptError::PayloadConflict)?;
    let month: i32 = parts[1]
        .parse()
        .map_err(|_| ReceiptError::PayloadConflict)?;
    let day: i32 = parts[2]
        .parse()
        .map_err(|_| ReceiptError::PayloadConflict)?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(ReceiptError::PayloadConflict);
    }
    Ok(year * 372 + month * 31 + day)
}

/// Capacity inspection records an observed value and the next action. It does not claim a scale test.
pub fn admit_capacity_observation(observed: i64, next_action: &str) -> Result<(), ReceiptError> {
    if observed < 0 || next_action.trim().is_empty() {
        return Err(ReceiptError::PayloadConflict);
    }
    Ok(())
}

/// Archive only a task whose stored status is already finished. A caller-supplied status is not enough.
pub fn archive_matches_stored(
    stored_status: &str,
    requested_status: &str,
    object_kind: &str,
) -> Result<&'static str, ReceiptError> {
    if stored_status != requested_status {
        return Err(ReceiptError::PayloadConflict);
    }
    archive_task_decision(stored_status, object_kind)
}

/// Finished tasks may be archived. Unknown commands, recovery artifacts, and business rows stay.
pub fn archive_task_decision(
    status: &str,
    object_kind: &str,
) -> Result<&'static str, ReceiptError> {
    if matches!(object_kind, "UNKNOWN_COMMAND" | "RECOVERY" | "BUSINESS") {
        return Ok("KEPT");
    }
    if object_kind == "TASK" && matches!(status, "SUCCEEDED" | "FAILED" | "CANCELLED") {
        return Ok("ARCHIVED");
    }
    if object_kind == "TASK" {
        return Ok("KEPT");
    }
    Err(ReceiptError::PayloadConflict)
}

/// Offline work may keep a draft. It must not claim a save or invent available stock.
pub fn offline_client_decision(online: bool, action: &str) -> Result<&'static str, ReceiptError> {
    match action {
        "DRAFT" => Ok("DRAFT_KEPT"),
        "SAVE" | "STOCK" if online => Ok("SERVER"),
        "SAVE" | "STOCK" => Ok("REFUSED"),
        _ => Err(ReceiptError::PayloadConflict),
    }
}

pub const HEAVY_TASK_LIMIT: i32 = 2;
pub const GLOBAL_HEAVY_TASK_LIMIT: i32 = 2;
pub const ORDER_ENTRY_RESERVE: i32 = 4;

/// Backup age is the seconds between the stored cutoff and the observation time.
pub fn backup_age_seconds(cutoff_epoch: i64, observed_epoch: i64) -> Result<i64, ReceiptError> {
    if cutoff_epoch <= 0 || observed_epoch < cutoff_epoch {
        return Err(ReceiptError::PayloadConflict);
    }
    Ok(observed_epoch - cutoff_epoch)
}

/// A refused heavy command stays queryable. It is not dropped from the admission record.
pub fn refused_work_stays_queryable(status: &str, command_id: &str) -> Result<(), ReceiptError> {
    if status == "429" && !command_id.trim().is_empty() {
        Ok(())
    } else {
        Err(ReceiptError::PayloadConflict)
    }
}

/// A live process is not the same fact as a reachable database.
pub fn probe_liveness(
    process_alive: bool,
    database_reachable: bool,
) -> (&'static str, &'static str) {
    (
        if process_alive { "ok" } else { "down" },
        if database_reachable { "ok" } else { "down" },
    )
}

/// Deep health text names both facts. Database loss does not hide a live process.
pub fn deep_health_body(process_alive: bool, database_reachable: bool) -> &'static str {
    match probe_liveness(process_alive, database_reachable) {
        ("ok", "ok") => "ok",
        ("ok", "down") => "process ok; db not ready",
        _ => "down",
    }
}

/// Process liveness, database, delivery delay, backlog, backup age, and capacity stay separate.
pub fn health_signal(
    kind: &str,
    value: i64,
    threshold: i64,
    duration_seconds: i64,
) -> Result<&'static str, ReceiptError> {
    if !matches!(
        kind,
        "PROCESS" | "DATABASE" | "DELIVERY_DELAY" | "BACKLOG" | "BACKUP_AGE" | "CAPACITY"
    ) {
        return Err(ReceiptError::PayloadConflict);
    }
    if value < 0 || threshold < 0 || duration_seconds < 0 {
        return Err(ReceiptError::PayloadConflict);
    }
    if kind == "PROCESS" || kind == "DATABASE" {
        return Ok(if value == 1 { "OK" } else { "DOWN" });
    }
    Ok(if value > threshold && duration_seconds > 0 {
        "ALERT"
    } else {
        "OK"
    })
}

/// A full heavy queue returns 429 or queues with a retry hint. Ordinary order entry keeps reserved slots.
/// The global heavy count is a separate cap. Filling one tenant does not by itself fill the global cap.
pub fn admit_work(
    kind: &str,
    heavy_running: i32,
    order_running: i32,
    global_heavy: i32,
) -> Result<&'static str, ReceiptError> {
    if heavy_running < 0 || order_running < 0 || global_heavy < 0 {
        return Err(ReceiptError::PayloadConflict);
    }
    match kind {
        "HEAVY" if heavy_running >= HEAVY_TASK_LIMIT || global_heavy >= GLOBAL_HEAVY_TASK_LIMIT => Ok("429"),
        "HEAVY" => Ok("QUEUED"),
        "ORDER" if order_running >= ORDER_ENTRY_RESERVE => Ok("429"),
        "ORDER" => Ok("ACCEPTED"),
        _ => Err(ReceiptError::PayloadConflict),
    }
}

pub const MAX_TRANSIENT_RETRIES: i32 = 5;
pub const RETRY_BACKOFF_SECONDS: [i64; 5] = [1, 2, 4, 8, 16];

/// Task, event, and business-effect identities stay distinct.
pub fn admit_task_identity(
    task_id: &str,
    event_id: &str,
    effect_id: &str,
) -> Result<(), ReceiptError> {
    if task_id.trim().is_empty() || event_id.trim().is_empty() || effect_id.trim().is_empty() {
        return Err(ReceiptError::EmptyRequest);
    }
    if task_id == event_id || task_id == effect_id || event_id == effect_id {
        return Err(ReceiptError::PayloadConflict);
    }
    Ok(())
}

/// A transient error may be retried five times. A business error stops. An unknown result waits.
/// `attempt_count` is the count after this failure, not the count before it.
pub fn classify_task_failure(
    attempt_count: i32,
    transient: bool,
    known: bool,
) -> Result<&'static str, ReceiptError> {
    if !known {
        return Ok("RECONCILING");
    }
    if !transient {
        return Ok("FAILED");
    }
    if attempt_count >= MAX_TRANSIENT_RETRIES {
        return Ok("DEAD");
    }
    if attempt_count < 1 {
        return Err(ReceiptError::PayloadConflict);
    }
    Ok("QUEUED")
}

pub fn retry_backoff_seconds(next_attempt: i32) -> Result<i64, ReceiptError> {
    if !(1..=MAX_TRANSIENT_RETRIES).contains(&next_attempt) {
        return Err(ReceiptError::PayloadConflict);
    }
    Ok(RETRY_BACKOFF_SECONDS[(next_attempt - 1) as usize])
}

/// A posted issue or payment is not erased by cancel. It needs a reversing document.
pub fn cancel_at_checkpoint(posted: bool) -> &'static str {
    if posted {
        "REVERSE_REQUIRED"
    } else {
        "CANCELLED"
    }
}

/// A repeated or late message keeps the original effect. It cannot open a second effect.
pub fn duplicate_delivery_keeps_effect(
    stored_effect: Option<&str>,
) -> Result<&'static str, ReceiptError> {
    match stored_effect {
        Some(effect) if !effect.trim().is_empty() => Ok("KEEP"),
        _ => Err(ReceiptError::EmptyRequest),
    }
}

pub fn failed(request_id: &str, payload_hash: &str, effect_id: &str) -> CommandReceipt {
    CommandReceipt {
        request_id: request_id.trim().to_string(),
        payload_hash: payload_hash.to_string(),
        status: CommandStatus::Failed,
        effect_id: effect_id.to_string(),
    }
}

pub fn succeeded(request_id: &str, payload_hash: &str, effect_id: &str) -> CommandReceipt {
    CommandReceipt {
        request_id: request_id.trim().to_string(),
        payload_hash: payload_hash.to_string(),
        status: CommandStatus::Succeeded,
        effect_id: effect_id.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_liveness_stays_separate_from_database() {
        assert_eq!(probe_liveness(true, false), ("ok", "down"));
        assert_eq!(deep_health_body(true, false), "process ok; db not ready");
        assert_eq!(deep_health_body(true, true), "ok");
    }

    #[test]
    fn same_command_returns_the_original_effect() {
        let stored = succeeded("req-1", "hash-a", "effect-1");
        let replay = admit_retry(Some(&stored), "req-1", "hash-a")
            .unwrap()
            .unwrap();
        assert_eq!(replay.effect_id, "effect-1");
        assert!(replay.status.is_complete());
    }

    #[test]
    fn inbound_retry_keeps_one_ledger() {
        let payload = inbound_payload(
            "11111111-1111-1111-1111-111111111111",
            "22222222-2222-2222-2222-222222222222",
            1,
            "sample",
        );
        let stored = succeeded("in-1", &payload, "ledger-1");
        let replay = admit_inbound_retry(
            Some(&stored),
            "in-1",
            "11111111-1111-1111-1111-111111111111",
            "22222222-2222-2222-2222-222222222222",
            1,
            "sample",
        )
        .unwrap()
        .unwrap();
        assert_eq!(replay.effect_id, "ledger-1");
        assert_eq!(
            admit_inbound_retry(
                Some(&stored),
                "in-1",
                "11111111-1111-1111-1111-111111111111",
                "22222222-2222-2222-2222-222222222222",
                2,
                "sample",
            ),
            Err(ReceiptError::PayloadConflict)
        );
    }

    #[test]
    fn different_payload_does_not_create_a_second_effect() {
        let stored = succeeded("req-1", "hash-a", "effect-1");
        assert_eq!(
            admit_retry(Some(&stored), "req-1", "hash-b"),
            Err(ReceiptError::PayloadConflict)
        );
    }
}
