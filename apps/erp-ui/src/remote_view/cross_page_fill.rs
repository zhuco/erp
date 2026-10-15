//! Bounded cross-page product fill over a private server-frozen query.
//! SKU ranges use a separate bounded repeatable-read loader in details_fill.
use super::Session;
use crate::{
    http::{HttpError, HttpRequest, HttpTask},
    remote_paste::{Preview, plan_indexed_fill},
    remote_selection,
};
use eframe::egui;
use erp_grid_core::{
    CellRect, Column, RowId,
    remote::{PageAddress, PageRequest, RemotePage, RemoteQuery, RemoteRow},
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const MAX_TARGET_ROWS: usize = 1_000;
const MAX_TOTAL_BYTES: usize = 8 * 1024 * 1024;
const PAGE_ROWS: usize = 500;

#[derive(Default)]
pub(super) struct CrossPageFillRange {
    pub(super) open: bool,
    first: String,
    last: String,
    error: Option<String>,
}

impl CrossPageFillRange {
    pub(super) fn open(&mut self) {
        self.open = true;
        self.error = None;
    }

    /// Returns an inclusive, one-based row range only after the user presses Prepare.
    pub(super) fn show(&mut self, ctx: &egui::Context, id: egui::Id) -> Option<(u64, u64)> {
        let mut prepared = None;
        let mut open = self.open;
        egui::Window::new("跨页填充")
            .id(id)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("行范围");
                    ui.add(egui::TextEdit::singleline(&mut self.first).desired_width(72.0));
                    ui.label("–");
                    ui.add(egui::TextEdit::singleline(&mut self.last).desired_width(72.0));
                });
                if let Some(error) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                if ui.button("准备预览").clicked() {
                    let parsed = self
                        .first
                        .parse::<u64>()
                        .map_err(|_| "起始行必须是正整数".to_owned())
                        .and_then(|first| {
                            self.last
                                .parse::<u64>()
                                .map_err(|_| "结束行必须是正整数".to_owned())
                                .and_then(|last| validate_range(first, last))
                        });
                    match parsed {
                        Ok(range) => prepared = Some(range),
                        Err(error) => self.error = Some(error),
                    }
                }
            });
        self.open = open && prepared.is_none();
        prepared
    }
}

struct SourceCapture {
    rect: CellRect,
    rows: Vec<(RowId, Option<String>)>,
}

struct PageJob {
    request: PageRequest,
    task: HttpTask,
}

pub(super) struct CrossPageFill {
    generation: u64,
    query: RemoteQuery,
    source: SourceCapture,
    target: CellRect,
    columns: Vec<Column>,
    issuance: Option<HttpTask>,
    credential: Option<remote_selection::Credential>,
    queued: VecDeque<PageRequest>,
    jobs: Vec<PageJob>,
    pages: BTreeMap<u64, RemotePage>,
    preview: Option<Preview>,
    error: Option<String>,
    ticket: u64,
    received_bytes: usize,
    authentication_failed: bool,
}

impl CrossPageFill {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn start(
        session: &Session,
        query: &RemoteQuery,
        generation: u64,
        current_page_start: u64,
        source: CellRect,
        current_rows: &[RemoteRow],
        columns: &[Column],
        target_first_one_based: u64,
        target_last_one_based: u64,
        ctx: &egui::Context,
    ) -> Result<Self, String> {
        let (target_first_one_based, target_last_one_based) =
            validate_range(target_first_one_based, target_last_one_based)?;
        let target_start = target_first_one_based - 1;
        // An inclusive one-based end is also the exclusive zero-based end.
        let target_end = target_last_one_based;
        if source.bottom > current_rows.len()
            || source.top >= source.bottom
            || source.left >= source.right
            || source.right > columns.len()
        {
            return Err("Fill source is outside the current loaded page".into());
        }
        let local_source = source;
        let source_top = current_page_start
            .checked_add(u64::try_from(local_source.top).map_err(|_| "Fill source row overflow")?)
            .ok_or("Fill source row overflow")?;
        let source_bottom = current_page_start
            .checked_add(
                u64::try_from(local_source.bottom).map_err(|_| "Fill source row overflow")?,
            )
            .ok_or("Fill source row overflow")?;
        let target = CellRect {
            top: usize::try_from(target_start).map_err(|_| "Fill target row overflow")?,
            left: source.left,
            bottom: usize::try_from(target_end).map_err(|_| "Fill target row overflow")?,
            right: source.right,
        };
        let source = CellRect {
            top: usize::try_from(source_top).map_err(|_| "Fill source row overflow")?,
            left: source.left,
            bottom: usize::try_from(source_bottom).map_err(|_| "Fill source row overflow")?,
            right: source.right,
        };
        crate::remote_paste::fill_axis(source, target)?;
        let captured = current_rows[local_source.top..local_source.bottom]
            .iter()
            .map(|row| (row.id, row.version.clone()))
            .collect::<Vec<_>>();
        if captured
            .iter()
            .any(|(id, version)| id.0 == 0 || version.as_deref().is_none_or(str::is_empty))
        {
            return Err("Fill source lacks stable identity or version".into());
        }
        let freeze_request = PageRequest {
            ticket: 0,
            generation,
            query: query.clone(),
            address: PageAddress::Offset { start: 0, limit: 1 },
        };
        let encoded = remote_selection::encode_freeze(session.tenant, &freeze_request)?;
        let issuance = session
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
            .map_err(|error| error.to_string())?;
        Ok(Self {
            generation,
            query: query.clone(),
            source: SourceCapture {
                rect: source,
                rows: captured,
            },
            target,
            columns: columns.to_vec(),
            issuance: Some(issuance),
            credential: None,
            queued: VecDeque::new(),
            jobs: Vec::new(),
            pages: BTreeMap::new(),
            preview: None,
            error: None,
            ticket: 0,
            received_bytes: 0,
            authentication_failed: false,
        })
    }

    pub(super) fn cancel(&mut self) {
        self.issuance = None;
        self.jobs.clear();
        self.queued.clear();
        self.credential = None;
        self.preview = None;
        self.error = Some("Cross-page fill cancelled".into());
    }

    pub(super) fn poll(
        &mut self,
        session: &Session,
        query: &RemoteQuery,
        generation: u64,
        ctx: &egui::Context,
        now: u64,
    ) {
        if &self.query != query || self.generation != generation {
            self.cancel();
            self.error = Some("Query or sort changed; prepare the fill again".into());
            return;
        }
        if let Some(result) = self.issuance.as_mut().and_then(HttpTask::poll) {
            self.issuance = None;
            let result = self
                .admit_response(result)
                .and_then(|bytes| remote_selection::decode_credential(&bytes, now));
            match result {
                Ok(credential) => {
                    if self.target.bottom as u64 > credential.total {
                        self.error = Some("Fill target exceeds the frozen query".into());
                        return;
                    }
                    self.enqueue_pages(&credential);
                    self.credential = Some(credential);
                }
                Err(error) => self.error = Some(error),
            }
        }
        if self
            .credential
            .as_ref()
            .is_some_and(|credential| now >= credential.expires_at)
        {
            self.cancel();
            self.error = Some("Frozen fill expired; prepare it again".into());
            return;
        }
        while self.jobs.len() < 2 {
            let Some(request) = self.queued.pop_front() else {
                break;
            };
            let Some(credential) = &self.credential else {
                break;
            };
            let encoded = match remote_selection::encode_query_order_page(
                session.tenant,
                credential,
                request.address_start(),
                request.address.limit() as u16,
                &self.query.sorts,
            ) {
                Ok(encoded) => encoded,
                Err(error) => {
                    self.error = Some(error);
                    self.jobs.clear();
                    self.queued.clear();
                    return;
                }
            };
            match session.client.start(
                HttpRequest {
                    path: encoded.path,
                    query: Vec::new(),
                    json: Some(encoded.body),
                    bearer: Some(session.token.clone()),
                    idempotency_key: None,
                },
                ctx,
            ) {
                Ok(task) => self.jobs.push(PageJob { request, task }),
                Err(error) => {
                    self.error = Some(error.to_string());
                    self.jobs.clear();
                    self.queued.clear();
                    return;
                }
            }
        }
        let mut index = 0;
        while index < self.jobs.len() {
            let Some(result) = self.jobs[index].task.poll() else {
                index += 1;
                continue;
            };
            let job = self.jobs.remove(index);
            let Some(credential) = &self.credential else {
                return;
            };
            let credential = credential.clone();
            let result = self.admit_response(result).and_then(|bytes| {
                remote_selection::decode_query_order_page(
                    session.tenant,
                    &credential,
                    &job.request,
                    &bytes,
                    now,
                )
            });
            match result {
                Ok(page) => {
                    self.pages.insert(job.request.address_start(), page);
                }
                Err(error) => {
                    self.error = Some(error);
                    self.jobs.clear();
                    self.queued.clear();
                    return;
                }
            }
        }
        if self.issuance.is_none()
            && self.jobs.is_empty()
            && self.queued.is_empty()
            && self.credential.is_some()
            && self.error.is_none()
            && self.preview.is_none()
        {
            if let Err(error) = self.finish() {
                self.error = Some(error);
            } else {
                // The preview now owns fixed identities, values and CAS versions; it no
                // longer depends on the short-lived page credential.
                self.credential = None;
            }
        }
        if self.issuance.is_some() || !self.jobs.is_empty() || !self.queued.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }

    pub(super) fn take_preview(&mut self) -> Option<Preview> {
        self.preview.take()
    }
    /// Column identities and labels captured when preparation started.
    pub(super) fn columns(&self) -> &[Column] {
        &self.columns
    }
    pub(super) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub(super) fn authentication_failed(&self) -> bool {
        self.authentication_failed
    }
    pub(super) fn pending(&self) -> bool {
        self.issuance.is_some() || !self.jobs.is_empty() || !self.queued.is_empty()
    }

    fn admit_response(&mut self, result: Result<Vec<u8>, HttpError>) -> Result<Vec<u8>, String> {
        let bytes = match result {
            Err(HttpError::Status(401 | 403)) => {
                self.authentication_failed = true;
                return Err("Session expired or product access denied".into());
            }
            Err(error) => return Err(error.to_string()),
            Ok(bytes) => bytes,
        };
        self.received_bytes = next_received_bytes(self.received_bytes, bytes.len())?;
        Ok(bytes)
    }

    fn enqueue_pages(&mut self, _credential: &remote_selection::Credential) {
        let ranges = [
            (self.source.rect.top, self.source.rect.bottom),
            (self.target.top, self.target.bottom),
        ];
        let mut seen = BTreeSet::new();
        for (start, length) in page_ranges(&ranges) {
            if seen.insert((start, length)) {
                self.ticket = self.ticket.wrapping_add(1);
                self.queued.push_back(PageRequest {
                    ticket: self.ticket,
                    generation: self.generation,
                    query: self.query.clone(),
                    address: PageAddress::Offset {
                        start: start as u64,
                        limit: length as u16,
                    },
                });
            }
        }
    }

    fn finish(&mut self) -> Result<(), String> {
        let bytes = self
            .pages
            .values()
            .flat_map(|page| &page.rows)
            .try_fold(0usize, |sum, row| {
                row.cells.values().chain(row.version.iter()).try_fold(
                    sum.checked_add(64 + row.cells.len() * 64)?,
                    |row_sum, value| row_sum.checked_add(value.len()),
                )
            })
            .ok_or("Frozen fill data size overflow")?;
        if bytes > MAX_TOTAL_BYTES {
            return Err("Frozen fill exceeds 8 MiB".into());
        }
        let source_rows = self.collect(self.source.rect.top, self.source.rect.bottom)?;
        for (row, (id, version)) in source_rows.iter().zip(&self.source.rows) {
            if row.id != *id || !same_version(row.version.as_deref(), version.as_deref()) {
                return Err("Frozen source changed; reselect the source rows".into());
            }
        }
        let targets = self
            .collect(self.target.top, self.target.bottom)?
            .into_iter()
            .enumerate()
            .map(|(offset, row)| (self.target.top + offset, row))
            .collect::<Vec<_>>();
        self.preview = Some(plan_indexed_fill(
            "products.update",
            self.source.rect,
            &source_rows,
            self.target,
            &targets,
            &self.columns,
        )?);
        Ok(())
    }

    fn collect(&self, start: usize, end: usize) -> Result<Vec<RemoteRow>, String> {
        let mut rows = Vec::with_capacity(end - start);
        let mut at = start;
        while at < end {
            let page = self
                .pages
                .get(&(at as u64))
                .ok_or("Frozen fill page is missing")?;
            let take = (end - at).min(page.rows.len());
            rows.extend(page.rows.iter().take(take).cloned());
            at += take;
            if take == 0 {
                return Err("Frozen fill page is empty".into());
            }
        }
        if rows.len() != end - start {
            return Err("Frozen fill rows are incomplete".into());
        }
        Ok(rows)
    }
}

trait RequestAddressStart {
    fn address_start(&self) -> u64;
}
impl RequestAddressStart for PageRequest {
    fn address_start(&self) -> u64 {
        match &self.address {
            PageAddress::Offset { start, .. } => *start,
            PageAddress::Cursor { .. } => 0,
        }
    }
}

fn validate_range(first: u64, last: u64) -> Result<(u64, u64), String> {
    if first == 0 {
        return Err("Fill start row must be positive".into());
    }
    last.checked_sub(first)
        .and_then(|difference| difference.checked_add(1))
        .and_then(|length| usize::try_from(length).ok())
        .filter(|length| (1..=MAX_TARGET_ROWS).contains(length))
        .ok_or("Cross-page fill must target an ordered range of 1–1,000 rows")?;
    Ok((first, last))
}

fn page_ranges(ranges: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut pages = Vec::new();
    for &(start, end) in ranges {
        let mut at = start;
        while at < end {
            let length = (end - at).min(PAGE_ROWS);
            pages.push((at, length));
            at += length;
        }
    }
    pages
}

fn next_received_bytes(received: usize, response: usize) -> Result<usize, String> {
    received
        .checked_add(response)
        .filter(|total| *total <= MAX_TOTAL_BYTES)
        .ok_or_else(|| "Frozen fill responses exceed 8 MiB".into())
}

// SQL jsonb and the normal API serialize the same UTC timestamp differently.
// Compare every fractional digit; millisecond conversion would lose CAS precision.
fn same_version(left: Option<&str>, right: Option<&str>) -> bool {
    fn utc_parts(value: &str) -> Option<(&str, &str)> {
        let utc = value
            .strip_suffix('Z')
            .or_else(|| value.strip_suffix("+00:00"))?;
        let (seconds, fraction) = utc.split_once('.').unwrap_or((utc, ""));
        Some((seconds, fraction.trim_end_matches('0')))
    }
    match (left, right) {
        (Some(left), Some(right)) if !left.is_empty() && !right.is_empty() => {
            left == right || utc_parts(left).is_some_and(|parts| Some(parts) == utc_parts(right))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_source_versions_preserve_submillisecond_precision() {
        assert!(same_version(
            Some("2026-09-10T08:42:27.100Z"),
            Some("2026-09-10T08:42:27.1+00:00")
        ));
        assert!(same_version(
            Some("2026-09-10T08:42:27Z"),
            Some("2026-09-10T08:42:27.000+00:00")
        ));
        assert!(!same_version(
            Some("2026-09-10T08:42:27.100001Z"),
            Some("2026-09-10T08:42:27.100002+00:00")
        ));
        assert!(!same_version(None, None));
    }

    #[test]
    fn range_is_one_based_inclusive_and_bounded() {
        assert_eq!(validate_range(1, 1).unwrap(), (1, 1));
        assert_eq!(validate_range(7, 1_006).unwrap(), (7, 1_006));
        assert!(validate_range(0, 1).is_err());
        assert!(validate_range(9, 8).is_err());
        assert!(validate_range(1, 1_001).is_err());
    }

    #[test]
    fn page_plan_fetches_only_requested_bounded_ranges() {
        assert_eq!(
            page_ranges(&[(40, 42), (1_000, 2_000)]),
            [(40, 2), (1_000, 500), (1_500, 500)]
        );
    }

    #[test]
    fn raw_response_budget_is_cumulative() {
        assert_eq!(
            next_received_bytes(MAX_TOTAL_BYTES - 1, 1).unwrap(),
            MAX_TOTAL_BYTES
        );
        assert!(next_received_bytes(MAX_TOTAL_BYTES, 1).is_err());
        assert!(next_received_bytes(usize::MAX, 1).is_err());
    }
}
