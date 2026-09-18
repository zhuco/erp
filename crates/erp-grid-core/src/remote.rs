//! Transport-independent remote page lifecycle. No I/O or ERP endpoint knowledge.
use crate::{ColumnId, Filter, RowId, SortRule};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

mod updates;
pub use updates::{RowPatch, UpdateQueue, UpdateReceipt};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataScope {
    pub source: String,
    pub schema: u64,
    /// Opaque non-secret identity including merchant, user and permission revision.
    pub authorization: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteQuery {
    pub expression: Option<Box<crate::paged_query::FilterExpression>>,
    pub scope: DataScope,
    pub filters: BTreeMap<ColumnId, Filter>,
    pub sorts: Vec<SortRule>,
    pub search: Option<String>,
}
impl RemoteQuery {
    /// Sorting changes page order, but not the explicit checked-ID universe.
    pub fn same_selection_scope(&self, other: &Self) -> bool {
        self.scope == other.scope
            && self.filters == other.filters
            && self.search == other.search
            && self.expression == other.expression
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageAddress {
    Offset { start: u64, limit: u16 },
    Cursor { after: Option<String>, limit: u16 },
}
impl PageAddress {
    pub fn limit(&self) -> usize {
        match self {
            Self::Offset { limit, .. } | Self::Cursor { limit, .. } => usize::from(*limit),
        }
    }
    fn validate(&self) -> Result<(), String> {
        if !(1..=500).contains(&self.limit()) {
            return Err("Page size must be 1–500".into());
        }
        match self {
            Self::Offset { start, limit } if start.checked_add(u64::from(*limit)).is_none() => {
                Err("Page range overflow".into())
            }
            Self::Cursor {
                after: Some(cursor),
                ..
            } if cursor.is_empty() || cursor.len() > 4096 => Err("Invalid cursor".into()),
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowCount {
    Exact(u64),
    Unknown,
}

#[derive(Clone, Debug)]
pub struct RemoteRow {
    pub id: RowId,
    pub version: Option<String>,
    pub cells: BTreeMap<ColumnId, String>,
}
#[derive(Clone, Debug)]
pub struct RemotePage {
    pub rows: Vec<RemoteRow>,
    pub total: RowCount,
    pub next_cursor: Option<String>,
    /// None explicitly means live pages without snapshot consistency.
    pub snapshot: Option<String>,
}
impl RemotePage {
    fn validate(&self, address: &PageAddress) -> Result<usize, String> {
        if self.rows.len() > address.limit() {
            return Err("Response exceeds requested page size".into());
        }
        let mut ids = BTreeSet::new();
        let mut bytes = 0usize;
        for row in &self.rows {
            if !ids.insert(row.id) {
                return Err("Duplicate row identity in page".into());
            }
            if row.cells.len() > 256 {
                return Err("Response column limit exceeded".into());
            }
            for value in row.cells.values().chain(row.version.iter()) {
                if value.len() > 65_536 {
                    return Err("Response value limit exceeded".into());
                }
                bytes = bytes.saturating_add(value.len());
            }
            bytes = bytes.saturating_add(64 + row.cells.len() * 64);
        }
        for value in self.next_cursor.iter().chain(self.snapshot.iter()) {
            if value.is_empty() || value.len() > 4096 {
                return Err("Invalid response cursor or snapshot".into());
            }
            bytes = bytes.saturating_add(value.len());
        }
        if let PageAddress::Cursor {
            after: Some(previous),
            ..
        } = address
            && self.next_cursor.as_ref() == Some(previous)
        {
            return Err("Cursor did not advance".into());
        }
        if self.rows.is_empty() && self.next_cursor.is_some() {
            return Err("Empty page cannot advance cursor".into());
        }
        if let RowCount::Exact(total) = self.total {
            if total < self.rows.len() as u64 {
                return Err("Total is smaller than returned page".into());
            }
            if let PageAddress::Offset { start, .. } = address {
                let expected = total.saturating_sub(*start).min(address.limit() as u64);
                if expected != self.rows.len() as u64 {
                    return Err("Page length disagrees with exact total".into());
                }
            }
        }
        if bytes > RemotePages::MAX_BYTES {
            return Err("Response exceeds cache byte budget".into());
        }
        Ok(bytes)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageRequest {
    pub ticket: u64,
    pub generation: u64,
    pub query: RemoteQuery,
    pub address: PageAddress,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageStart {
    Cached,
    Pending(u64),
    Fetch(PageRequest),
}
#[derive(Debug, PartialEq, Eq)]
pub enum PageAcceptance {
    Stored,
    Stale,
    Failed(String),
}
struct CachedPage {
    address: PageAddress,
    page: RemotePage,
    bytes: usize,
}

/// Retained core resources only. Byte accounting estimates cached page payloads;
/// this is not allocator RSS, transport task count, or GPU memory.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RemotePageStats {
    pub blocks: usize,
    pub rows: usize,
    pub estimated_page_bytes: usize,
    pub pending_requests: usize,
    pub errors: usize,
}

#[derive(Default)]
pub struct RemotePages {
    query: Option<RemoteQuery>,
    generation: u64,
    ticket: u64,
    pending: BTreeMap<u64, PageRequest>,
    cache: VecDeque<CachedPage>,
    errors: VecDeque<(PageAddress, String)>,
    snapshot: Option<Option<String>>,
}
impl RemotePages {
    pub const MAX_BLOCKS: usize = 10;
    pub const MAX_ROWS: usize = 1000;
    pub const MAX_BYTES: usize = 8 * 1024 * 1024;
    pub const MAX_IN_FLIGHT: usize = 2;

    /// Returns transport tickets to abort. Cancellation is advisory; late replies
    /// are rejected even if the transport cannot stop them.
    pub fn bind(&mut self, query: RemoteQuery) -> Vec<u64> {
        if self.query.as_ref() == Some(&query) {
            return Vec::new();
        }
        let cancelled = self.invalidate();
        self.query = Some(query);
        cancelled
    }
    pub fn invalidate(&mut self) -> Vec<u64> {
        self.generation = self.generation.wrapping_add(1);
        let cancelled = self.pending.keys().copied().collect();
        self.pending.clear();
        self.cache.clear();
        self.errors.clear();
        self.snapshot = None;
        cancelled
    }
    pub fn clear(&mut self) -> Vec<u64> {
        let cancelled = self.invalidate();
        self.query = None;
        cancelled
    }
    pub fn cancel(&mut self, ticket: u64) -> bool {
        self.pending.remove(&ticket).is_some()
    }
    pub fn start(&mut self, address: PageAddress) -> Result<PageStart, String> {
        address.validate()?;
        let query = self.query.clone().ok_or("Bind a query first")?;
        if let Some(index) = self.cache.iter().position(|p| p.address == address) {
            if let Some(cached) = self.cache.remove(index) {
                self.cache.push_back(cached);
            }
            return Ok(PageStart::Cached);
        }
        if let Some(request) = self.pending.values().find(|p| p.address == address) {
            return Ok(PageStart::Pending(request.ticket));
        }
        if self.pending.len() >= Self::MAX_IN_FLIGHT {
            return Err("Request capacity reached".into());
        }
        self.errors.retain(|(page, _)| page != &address);
        self.ticket = self.ticket.wrapping_add(1);
        let request = PageRequest {
            ticket: self.ticket,
            generation: self.generation,
            query,
            address,
        };
        self.pending.insert(request.ticket, request.clone());
        Ok(PageStart::Fetch(request))
    }
    pub fn accept(
        &mut self,
        request: &PageRequest,
        response: Result<RemotePage, String>,
    ) -> PageAcceptance {
        if self.pending.get(&request.ticket) != Some(request) {
            return PageAcceptance::Stale;
        }
        self.pending.remove(&request.ticket);
        let result = response.and_then(|page| {
            let bytes = page.validate(&request.address)?;
            if self
                .snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot != &page.snapshot)
            {
                return Err("Snapshot changed; refresh the query".into());
            }
            if let Some(cached) = self.cache.front()
                && cached.page.total != page.total
                && page.snapshot.is_some()
            {
                return Err("Snapshot total changed".into());
            }
            Ok((page, bytes))
        });
        match result {
            Ok((page, bytes)) => {
                self.snapshot = Some(page.snapshot.clone());
                self.cache.push_back(CachedPage {
                    address: request.address.clone(),
                    page,
                    bytes,
                });
                while self.cache.len() > Self::MAX_BLOCKS
                    || self.cached_rows() > Self::MAX_ROWS
                    || self.cache.iter().map(|p| p.bytes).sum::<usize>() > Self::MAX_BYTES
                {
                    self.cache.pop_front();
                }
                PageAcceptance::Stored
            }
            Err(error) => {
                let error: String = error.chars().take(1024).collect();
                self.errors
                    .push_back((request.address.clone(), error.clone()));
                while self.errors.len() > Self::MAX_BLOCKS {
                    self.errors.pop_front();
                }
                PageAcceptance::Failed(error)
            }
        }
    }
    pub fn page(&self, address: &PageAddress) -> Option<&RemotePage> {
        self.cache
            .iter()
            .find(|p| &p.address == address)
            .map(|p| &p.page)
    }
    pub fn error(&self, address: &PageAddress) -> Option<&str> {
        self.errors
            .iter()
            .find(|(p, _)| p == address)
            .map(|(_, error)| error.as_str())
    }
    pub fn cached_rows(&self) -> usize {
        self.cache.iter().map(|p| p.page.rows.len()).sum()
    }
    pub fn stats(&self) -> RemotePageStats {
        RemotePageStats {
            blocks: self.cache.len(),
            rows: self.cached_rows(),
            estimated_page_bytes: self.cache.iter().map(|page| page.bytes).sum(),
            pending_requests: self.pending.len(),
            errors: self.errors.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn query() -> RemoteQuery {
        RemoteQuery {
            expression: None,
            scope: DataScope {
                source: "products".into(),
                schema: 1,
                authorization: "merchant:user:permission-v1".into(),
            },
            filters: BTreeMap::new(),
            sorts: Vec::new(),
            search: None,
        }
    }
    #[test]
    fn twenty_query_switches_release_resources_and_reject_all_late_replies() {
        let mut pages = RemotePages::default();
        for cycle in 0..20 {
            let mut current = query();
            current.search = Some(format!("cycle-{cycle}"));
            pages.bind(current);
            assert_eq!(pages.stats(), RemotePageStats::default());
            for start in (0..1200).step_by(100) {
                let request = fetch(&mut pages, address(start));
                assert_eq!(
                    pages.accept(&request, Ok(reply(start))),
                    PageAcceptance::Stored
                );
                let stats = pages.stats();
                assert!(stats.blocks <= RemotePages::MAX_BLOCKS);
                assert!(stats.rows <= RemotePages::MAX_ROWS);
                assert!(stats.estimated_page_bytes <= RemotePages::MAX_BYTES);
            }
            let failed = fetch(&mut pages, address(1200));
            assert!(matches!(
                pages.accept(&failed, Err("test failure".into())),
                PageAcceptance::Failed(_)
            ));
            let first = fetch(&mut pages, address(1300));
            let second = fetch(&mut pages, address(1400));
            assert_eq!(pages.stats().pending_requests, 2);
            assert_eq!(pages.stats().errors, 1);
            assert!(pages.stats().estimated_page_bytes > 0);
            let cancelled = pages.clear();
            assert_eq!(cancelled, [first.ticket, second.ticket]);
            assert_eq!(pages.stats(), RemotePageStats::default());
            for request in [&first, &second] {
                assert_eq!(
                    pages.accept(request, Ok(reply(1300))),
                    PageAcceptance::Stale
                );
            }
            assert_eq!(pages.stats(), RemotePageStats::default());
        }
    }
    #[test]
    fn expression_change_invalidates_selection_scope_but_sort_does_not() {
        let first = query();
        let mut next = first.clone();
        next.sorts.push(SortRule {
            column: ColumnId(0),
            descending: true,
        });
        assert!(first.same_selection_scope(&next));
        next.expression = Some(Box::new(crate::paged_query::FilterExpression::Condition {
            column: ColumnId(0),
            filter: Filter::Text("new scope".into()),
        }));
        assert!(!first.same_selection_scope(&next));
    }
    fn address(start: u64) -> PageAddress {
        PageAddress::Offset { start, limit: 100 }
    }
    fn fetch(pages: &mut RemotePages, address: PageAddress) -> PageRequest {
        match pages.start(address).unwrap() {
            PageStart::Fetch(request) => request,
            other => panic!("Expected fetch, got {other:?}"),
        }
    }
    fn reply(start: u64) -> RemotePage {
        RemotePage {
            rows: (start..start + 100)
                .map(|id| RemoteRow {
                    id: RowId(u128::from(id)),
                    version: Some("1".into()),
                    cells: BTreeMap::from([(ColumnId(1), format!("Product {id}"))]),
                })
                .collect(),
            total: RowCount::Exact(10_000),
            next_cursor: None,
            snapshot: Some("snapshot-1".into()),
        }
    }
    #[test]
    fn ten_thousand_matches_only_requests_one_hundred_and_cache_is_bounded() {
        let mut pages = RemotePages::default();
        pages.bind(query());
        for start in (0..1200).step_by(100) {
            let request = fetch(&mut pages, address(start));
            assert_eq!(request.address.limit(), 100);
            assert_eq!(
                pages.accept(&request, Ok(reply(start))),
                PageAcceptance::Stored
            );
            assert!(pages.cached_rows() <= 1000);
            assert!(pages.cache.len() <= 10);
        }
        assert!(pages.page(&address(0)).is_none());
        assert!(matches!(
            pages.start(address(1100)).unwrap(),
            PageStart::Cached
        ));
        assert_eq!(
            pages.page(&address(1100)).unwrap().total,
            RowCount::Exact(10_000)
        );
    }
    #[test]
    fn query_change_cancel_and_retry_reject_late_responses() {
        let mut pages = RemotePages::default();
        pages.bind(query());
        let old = fetch(&mut pages, address(0));
        assert_eq!(
            pages.start(address(0)).unwrap(),
            PageStart::Pending(old.ticket)
        );
        let mut changed = query();
        changed.scope.authorization = "other-merchant".into();
        assert_eq!(pages.bind(changed), [old.ticket]);
        let fresh = fetch(&mut pages, address(0));
        assert_eq!(pages.accept(&old, Ok(reply(0))), PageAcceptance::Stale);
        assert!(matches!(
            pages.accept(&fresh, Err("Network unavailable".into())),
            PageAcceptance::Failed(_)
        ));
        assert!(pages.page(&address(0)).is_none());
        assert_eq!(pages.error(&address(0)), Some("Network unavailable"));
        let retried = fetch(&mut pages, address(0));
        assert_ne!(fresh.ticket, retried.ticket);
        assert!(pages.error(&address(0)).is_none());
        assert_eq!(pages.accept(&fresh, Ok(reply(0))), PageAcceptance::Stale);
        assert_eq!(pages.accept(&retried, Ok(reply(0))), PageAcceptance::Stored);
        pages.clear();
        assert!(pages.start(address(0)).is_err());
    }
    #[test]
    fn unknown_count_cursor_preserves_unknown_and_rejects_nonadvancing_response() {
        let mut pages = RemotePages::default();
        pages.bind(query());
        let address = PageAddress::Cursor {
            after: Some("cursor-1".into()),
            limit: 100,
        };
        let request = fetch(&mut pages, address.clone());
        let mut page = reply(0);
        page.total = RowCount::Unknown;
        page.snapshot = None;
        page.next_cursor = Some("cursor-1".into());
        assert!(matches!(
            pages.accept(&request, Ok(page.clone())),
            PageAcceptance::Failed(_)
        ));
        let request = fetch(&mut pages, address.clone());
        page.next_cursor = Some("cursor-2".into());
        assert_eq!(pages.accept(&request, Ok(page)), PageAcceptance::Stored);
        assert_eq!(pages.page(&address).unwrap().total, RowCount::Unknown);
    }
    #[test]
    fn duplicate_ids_incorrect_counts_and_snapshot_changes_do_not_publish() {
        for variant in 0..4 {
            let mut pages = RemotePages::default();
            pages.bind(query());
            let first = fetch(&mut pages, address(0));
            pages.accept(&first, Ok(reply(0)));
            let next = fetch(&mut pages, address(100));
            let mut page = reply(100);
            match variant {
                0 => page.rows[1].id = page.rows[0].id,
                1 => {
                    page.rows.pop();
                }
                2 => page.snapshot = Some("other-snapshot".into()),
                _ => page.total = RowCount::Exact(11_000),
            }
            assert!(matches!(
                pages.accept(&next, Ok(page)),
                PageAcceptance::Failed(_)
            ));
            assert!(pages.page(&address(100)).is_none());
            assert_eq!(pages.cached_rows(), 100);
        }
    }
    #[test]
    fn selection_scope_excludes_sort_but_includes_filters_and_permissions() {
        let original = query();
        let mut changed = original.clone();
        changed.sorts.push(SortRule {
            column: ColumnId(1),
            descending: true,
        });
        assert!(original.same_selection_scope(&changed));
        changed.scope.schema += 1;
        assert!(!original.same_selection_scope(&changed));
        changed = original.clone();
        changed
            .filters
            .insert(ColumnId(1), Filter::Text("search".into()));
        assert!(!original.same_selection_scope(&changed));
    }
    #[test]
    fn requests_and_response_memory_are_bounded() {
        let mut pages = RemotePages::default();
        pages.bind(query());
        let first = fetch(&mut pages, address(0));
        let second = fetch(&mut pages, address(100));
        assert!(pages.start(address(200)).is_err());
        assert!(pages.cancel(second.ticket));
        assert_eq!(pages.accept(&second, Ok(reply(100))), PageAcceptance::Stale);
        let mut page = reply(0);
        page.rows[0].cells.insert(ColumnId(2), "x".repeat(65_537));
        assert!(matches!(
            pages.accept(&first, Ok(page)),
            PageAcceptance::Failed(_)
        ));
        assert!(
            pages
                .start(PageAddress::Offset {
                    start: u64::MAX,
                    limit: 100
                })
                .is_err()
        );
        assert!(
            pages
                .start(PageAddress::Offset { start: 0, limit: 0 })
                .is_err()
        );
    }
    #[test]
    fn uuid_id_keeps_high_bits_and_uses_lossless_text() {
        let left = RowId::from_uuid("00000000-0000-0001-0000-000000000001").unwrap();
        let right = RowId::from_uuid("00000000-0000-0002-0000-000000000001").unwrap();
        assert_ne!(left, right);
        assert_eq!(RowId::from_uuid(&left.to_uuid()), Some(left));
        assert!(RowId::from_uuid("00000000000000010000000000000001").is_none());
    }
}
