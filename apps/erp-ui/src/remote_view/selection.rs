use super::Session;
use crate::{
    http::{HttpError, HttpRequest, HttpTask},
    remote_selection::{self, Credential},
};
use eframe::egui;
use erp_grid_core::{
    RowId,
    matched_selection::{MatchedSelection, MatchingCredential, TrustedLoadedRows},
    remote::PageRequest,
};
use std::{collections::BTreeSet, time::Duration};

const MAX_PAGE_ROWS: usize = 500;
#[cfg(test)]
const MAX_EXCLUDED: usize = MatchedSelection::MAX_EXCLUDED;

struct CountTask {
    task: HttpTask,
    token: RowId,
    revision: u64,
    excluded_len: usize,
}

struct Counted {
    token: RowId,
    revision: u64,
    value: u64,
}

#[derive(Default)]
pub(super) struct FrozenSelection {
    tenant: Option<RowId>,
    binding: Option<(String, String)>,
    credential: Option<Credential>,
    matched: MatchedSelection,
    empty_exclusions: BTreeSet<RowId>,
    issuance: Option<HttpTask>,
    count_task: Option<CountTask>,
    counted: Option<Counted>,
    revision: u64,
    notice: Option<String>,
}

impl FrozenSelection {
    pub(super) fn suspend_reads(&mut self) {
        if self.issuance.take().is_some() {
            self.invalidate("Frozen selection request cancelled on page leave");
        } else if self.count_task.take().is_some() {
            self.counted = None;
            self.notice = Some("Selection count cancelled; retry count before acting".into());
        }
    }
    pub(super) fn start(
        &mut self,
        session: &Session,
        request: &PageRequest,
        ctx: &egui::Context,
        _now: u64,
    ) -> Result<(), String> {
        self.clear();
        let encoded = remote_selection::encode_freeze(session.tenant, request)?;
        let task = session
            .client
            .start(
                HttpRequest {
                    path: encoded.path,
                    query: Vec::new(),
                    json: Some(encoded.body),
                    bearer: Some(session.token.clone()),
                    idempotency_key: None,
                },
                ctx,
            )
            .map_err(sanitized_error)?;
        self.tenant = Some(session.tenant);
        let scope = format!(
            "{}:{}",
            request.query.scope.source, request.query.scope.authorization
        );
        self.binding = Some((scope.clone(), String::new()));
        self.matched
            .bind(&scope, "pending")
            .map_err(|_| "Invalid frozen selection context".to_owned())?;
        self.issuance = Some(task);
        self.notice = None;
        Ok(())
    }

    pub(super) fn poll(&mut self, ctx: &egui::Context, now: u64) {
        if self
            .credential
            .as_ref()
            .is_some_and(|credential| now >= credential.expires_at)
        {
            self.invalidate("Frozen selection expired");
            return;
        }

        let issuance_result = self.issuance.as_mut().and_then(HttpTask::poll);
        if let Some(result) = issuance_result {
            self.issuance = None;
            match result {
                Ok(bytes) => match remote_selection::decode_credential(&bytes, now) {
                    Ok(credential) => {
                        let total = credential.total;
                        let token = credential.token;
                        let Some((scope, _)) = self.binding.clone() else {
                            self.invalidate("Missing frozen selection context");
                            return;
                        };
                        self.binding = Some((scope.clone(), credential.query_fingerprint.clone()));
                        let matching = MatchingCredential {
                            token: token.to_uuid(),
                            authorization_scope: scope.clone(),
                            query_fingerprint: credential.query_fingerprint.clone(),
                            snapshot: token.to_uuid(),
                            total: total as usize,
                            expires_at: credential.expires_at,
                        };
                        if self
                            .matched
                            .bind(&scope, &credential.query_fingerprint)
                            .is_err()
                            || self.matched.use_all_matching(matching, now).is_err()
                        {
                            self.invalidate("Frozen selection context mismatch");
                            return;
                        }
                        self.credential = Some(credential);
                        self.counted = Some(Counted {
                            token,
                            revision: self.revision,
                            value: total,
                        });
                        self.notice = None;
                    }
                    Err(error) => self.invalidate(error),
                },
                Err(error) if invalidates_credential(&error) => {
                    self.invalidate(expired_message(&error));
                }
                Err(error) => self.invalidate(sanitized_error(error)),
            }
        }

        if let Some(credential) = &self.credential {
            ctx.request_repaint_after(Duration::from_secs(
                credential.expires_at.saturating_sub(now).min(1),
            ));
        }

        let count_result = self.count_task.as_mut().and_then(|job| job.task.poll());
        let Some(result) = count_result else {
            return;
        };
        let Some(job) = self.count_task.take() else {
            return;
        };
        let Some(credential) = self.credential.as_ref() else {
            return;
        };
        if credential.token != job.token || self.revision != job.revision {
            return;
        }
        match result {
            Ok(bytes) => {
                match remote_selection::decode_count(&bytes, credential.total, job.excluded_len) {
                    Ok(value) => {
                        self.counted = Some(Counted {
                            token: job.token,
                            revision: job.revision,
                            value,
                        });
                        self.notice = None;
                    }
                    Err(error) => {
                        self.counted = None;
                        self.notice = Some(error);
                    }
                }
            }
            Err(error) if invalidates_credential(&error) => {
                self.invalidate(expired_message(&error));
            }
            Err(error) => {
                self.counted = None;
                self.notice = Some(sanitized_error(error));
            }
        }
    }

    pub(super) fn set_page_checked(
        &mut self,
        session: &Session,
        rows: &[RowId],
        checked: &BTreeSet<RowId>,
        ctx: &egui::Context,
        now: u64,
    ) -> Result<(), String> {
        if self.tenant != Some(session.tenant) {
            self.invalidate("Frozen selection belongs to another session");
            return Err("Frozen selection belongs to another session".into());
        }
        let credential = self
            .credential
            .as_ref()
            .ok_or_else(|| "No active frozen selection".to_owned())?;
        if now >= credential.expires_at {
            return Err("Frozen selection expired".into());
        }
        if self.issuance.is_some() {
            return Err("Frozen selection is still pending".into());
        }
        if rows.len() > MAX_PAGE_ROWS || rows.len() as u64 > credential.total {
            return Err("Invalid frozen selection page".into());
        }
        let (scope, query) = self
            .binding
            .as_ref()
            .ok_or_else(|| "No active frozen selection".to_owned())?;
        let loaded = TrustedLoadedRows::from_verified_response(
            scope.clone(),
            query.clone(),
            credential.token.to_uuid(),
            rows.iter().copied(),
        )
        .map_err(selection_error)?;
        let mut changes = Vec::with_capacity(rows.len());
        let current = self.matched.excluded_ids().cloned().unwrap_or_default();
        for row in rows {
            let proof = loaded.proof(*row).map_err(selection_error)?;
            changes.push((proof, !checked.contains(row)));
        }
        self.matched
            .set_excluded_batch(changes, now)
            .map_err(selection_error)?;
        if self.matched.excluded_ids().cloned().unwrap_or_default() == current {
            return Ok(());
        }
        self.revision = self.revision.wrapping_add(1);
        self.count_task = None;
        self.counted = None;
        self.notice = None;
        self.start_count(session, ctx, now)
    }

    /// Retry an unavailable count without changing the frozen membership.
    pub(super) fn retry_count(
        &mut self,
        session: &Session,
        ctx: &egui::Context,
        now: u64,
    ) -> Result<(), String> {
        if self.count_task.is_some() {
            return Ok(());
        }
        self.start_count(session, ctx, now)
    }

    fn start_count(
        &mut self,
        session: &Session,
        ctx: &egui::Context,
        now: u64,
    ) -> Result<(), String> {
        let credential = self
            .credential
            .as_ref()
            .ok_or_else(|| "No active frozen selection".to_owned())?;
        if self.tenant != Some(session.tenant) {
            self.invalidate("Frozen selection belongs to another session");
            return Err("Frozen selection belongs to another session".into());
        }
        if now >= credential.expires_at {
            self.invalidate("Frozen selection expired");
            return Err("Frozen selection expired".into());
        }
        let excluded: Vec<_> = self
            .matched
            .excluded_ids()
            .map_or_else(Vec::new, |ids| ids.iter().copied().collect());
        let encoded = remote_selection::encode_count(session.tenant, credential, &excluded)?;
        let task = match session.client.start(
            HttpRequest {
                path: encoded.path,
                query: Vec::new(),
                json: Some(encoded.body),
                bearer: Some(session.token.clone()),
                idempotency_key: None,
            },
            ctx,
        ) {
            Ok(task) => task,
            Err(error) => {
                let message = sanitized_error(error);
                self.counted = None;
                self.notice = Some(message.clone());
                return Err(message);
            }
        };
        self.count_task = Some(CountTask {
            task,
            token: credential.token,
            revision: self.revision,
            excluded_len: excluded.len(),
        });
        self.notice = None;
        Ok(())
    }

    pub(super) fn clear(&mut self) {
        self.tenant = None;
        self.credential = None;
        self.binding = None;
        self.matched.clear();
        self.issuance = None;
        self.count_task = None;
        self.counted = None;
        self.revision = self.revision.wrapping_add(1);
        self.notice = None;
    }

    fn invalidate(&mut self, message: impl Into<String>) {
        self.clear();
        self.notice = Some(message.into());
    }

    pub(super) fn credential(&self) -> Option<&Credential> {
        self.credential.as_ref()
    }

    pub(super) fn exclusions(&self) -> &BTreeSet<RowId> {
        self.matched
            .excluded_ids()
            .unwrap_or(&self.empty_exclusions)
    }

    pub(super) fn pending(&self) -> bool {
        self.issuance.is_some() || self.count_task.is_some()
    }

    pub(super) fn count(&self) -> Option<u64> {
        let credential = self.credential.as_ref()?;
        self.counted.as_ref().and_then(|counted| {
            (counted.token == credential.token && counted.revision == self.revision)
                .then_some(counted.value)
        })
    }

    pub(super) fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    pub(super) fn is_active(&self) -> bool {
        self.credential.is_some()
    }
}

/// Applies checkbox state only to identities from a server-verified frozen page.
/// Extra IDs in `checked` are deliberately ignored.
#[cfg(test)]
fn apply_page_checked(
    credential: Option<&Credential>,
    issuance_pending: bool,
    exclusions: &BTreeSet<RowId>,
    rows: &[RowId],
    checked: &BTreeSet<RowId>,
    now: u64,
) -> Result<BTreeSet<RowId>, String> {
    if issuance_pending {
        return Err("Frozen selection is still pending".into());
    }
    let credential = credential.ok_or_else(|| "No active frozen selection".to_owned())?;
    if now >= credential.expires_at {
        return Err("Frozen selection expired".into());
    }
    if rows.len() > MAX_PAGE_ROWS || rows.len() as u64 > credential.total {
        return Err("Invalid frozen selection page".into());
    }
    let mut verified = BTreeSet::new();
    for row in rows {
        if row.0 == 0 || !verified.insert(*row) {
            return Err("Invalid frozen selection page".into());
        }
    }
    let scope = "fixture-scope";
    let matching = MatchingCredential {
        token: credential.token.to_uuid(),
        authorization_scope: scope.into(),
        query_fingerprint: credential.query_fingerprint.clone(),
        snapshot: credential.token.to_uuid(),
        total: credential.total as usize,
        expires_at: credential.expires_at,
    };
    let mut selection = MatchedSelection::default();
    selection
        .bind(scope, &credential.query_fingerprint)
        .map_err(selection_error)?;
    selection
        .use_all_matching(matching, now)
        .map_err(selection_error)?;
    for row in exclusions {
        let proof = TrustedLoadedRows::from_verified_response(
            scope.into(),
            credential.query_fingerprint.clone(),
            credential.token.to_uuid(),
            [*row],
        )
        .map_err(selection_error)?
        .proof(*row)
        .map_err(selection_error)?;
        selection
            .set_excluded(proof, true, now)
            .map_err(selection_error)?;
    }
    let loaded = TrustedLoadedRows::from_verified_response(
        scope.into(),
        credential.query_fingerprint.clone(),
        credential.token.to_uuid(),
        verified,
    )
    .map_err(selection_error)?;
    let changes = rows
        .iter()
        .map(|row| {
            loaded
                .proof(*row)
                .map(|proof| (proof, !checked.contains(row)))
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(selection_error)?;
    selection
        .set_excluded_batch(changes, now)
        .map_err(selection_error)?;
    Ok(selection.excluded_ids().cloned().unwrap_or_default())
}

fn invalidates_credential(error: &HttpError) -> bool {
    matches!(error, HttpError::Status(401 | 403 | 410))
}

fn expired_message(error: &HttpError) -> &'static str {
    match error {
        HttpError::Status(410) => "Frozen selection expired",
        _ => "Session expired or access denied",
    }
}

fn sanitized_error(error: HttpError) -> String {
    match error {
        HttpError::Status(code) => format!("Selection request failed (HTTP {code})"),
        HttpError::TooLarge => "Selection response is too large".into(),
        HttpError::InvalidAddress => "Invalid selection server address".into(),
        HttpError::Transport => "Selection request failed or timed out".into(),
        HttpError::Cancelled => "Selection request was cancelled".into(),
    }
}

fn selection_error(error: erp_grid_core::matched_selection::SelectionError) -> String {
    use erp_grid_core::matched_selection::SelectionError;
    match error {
        SelectionError::Expired => "Frozen selection expired".into(),
        SelectionError::ScopeMismatch | SelectionError::QueryMismatch => {
            "Frozen selection context mismatch".into()
        }
        SelectionError::SnapshotMismatch => "Frozen selection snapshot mismatch".into(),
        SelectionError::TooManyExcluded => "At most 10,000 frozen rows may be excluded".into(),
        SelectionError::ExclusionNotLoaded => "Invalid frozen selection page".into(),
        _ => "Invalid frozen selection".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credential(total: u64, expires_at: u64) -> Credential {
        Credential {
            token: RowId(1),
            query_fingerprint: "0123456789abcdef0123456789abcdef".into(),
            total,
            expires_at,
        }
    }

    #[test]
    fn leaving_cancels_count_without_discarding_frozen_identity() {
        let ctx = egui::Context::default();
        let client = crate::http::HttpClient::new("http://127.0.0.1:1").unwrap();
        let task = || {
            client
                .start(
                    HttpRequest {
                        path: "/fixture".into(),
                        query: vec![],
                        json: None,
                        bearer: None,
                        idempotency_key: None,
                    },
                    &ctx,
                )
                .unwrap()
        };
        let mut selection = FrozenSelection {
            credential: Some(credential(100, 20)),
            binding: Some(("scope".into(), "query".into())),
            count_task: Some(CountTask {
                task: task(),
                token: RowId(1),
                revision: 4,
                excluded_len: 0,
            }),
            revision: 4,
            ..Default::default()
        };
        selection.suspend_reads();
        assert!(!selection.pending());
        assert_eq!(selection.credential().unwrap().token, RowId(1));
        assert_eq!(selection.binding, Some(("scope".into(), "query".into())));
        assert_eq!(selection.revision, 4);
        assert_eq!(selection.count(), None);
        assert!(selection.notice().unwrap().contains("retry count"));
        selection.issuance = Some(task());
        selection.suspend_reads();
        assert!(!selection.pending());
        assert!(selection.credential().is_none());
        assert!(selection.notice().unwrap().contains("cancelled"));
    }

    #[test]
    fn pending_missing_and_expired_states_do_not_change_exclusions() {
        let original = BTreeSet::from([RowId(7)]);
        let checked = BTreeSet::new();
        assert!(apply_page_checked(None, true, &original, &[RowId(8)], &checked, 10).is_err());
        assert!(apply_page_checked(None, false, &original, &[RowId(8)], &checked, 10).is_err());
        assert!(
            apply_page_checked(
                Some(&credential(10, 10)),
                false,
                &original,
                &[RowId(8)],
                &checked,
                10
            )
            .is_err()
        );
        assert_eq!(original, BTreeSet::from([RowId(7)]));
    }

    #[test]
    fn page_checks_add_and_remove_only_verified_page_exclusions() {
        let credential = credential(100, 20);
        let original = BTreeSet::from([RowId(2), RowId(90)]);
        let checked = BTreeSet::from([RowId(2), RowId(999)]);
        assert_eq!(
            apply_page_checked(
                Some(&credential),
                false,
                &original,
                &[RowId(1), RowId(2), RowId(3)],
                &checked,
                10,
            ),
            Ok(BTreeSet::from([RowId(1), RowId(3), RowId(90)]))
        );
    }

    #[test]
    fn invalid_pages_and_exclusion_limit_fail_atomically() {
        let credential = credential(20_000, 20);
        let checked = BTreeSet::new();
        assert!(
            apply_page_checked(
                Some(&credential),
                false,
                &BTreeSet::new(),
                &[RowId(1), RowId(1)],
                &checked,
                10,
            )
            .is_err()
        );

        let full: BTreeSet<_> = (1..=MAX_EXCLUDED as u128).map(RowId).collect();
        assert!(
            apply_page_checked(
                Some(&credential),
                false,
                &full,
                &[RowId(MAX_EXCLUDED as u128 + 1)],
                &checked,
                10,
            )
            .is_err()
        );
        assert_eq!(full.len(), MAX_EXCLUDED);
    }

    #[test]
    fn count_is_visible_only_for_the_current_credential_revision() {
        let mut selection = FrozenSelection {
            credential: Some(credential(100, 20)),
            counted: Some(Counted {
                token: RowId(1),
                revision: 0,
                value: 97,
            }),
            ..Default::default()
        };
        assert_eq!(selection.count(), Some(97));
        selection.revision = 1;
        assert_eq!(selection.count(), None);
        selection.revision = 0;
        selection.credential = Some(Credential {
            token: RowId(2),
            ..credential(100, 20)
        });
        assert_eq!(selection.count(), None);
    }
}
