//! Atomic patches to already loaded live pages. Transport must deliver an ordered
//! version chain; opaque versions are compared for equality, never lexically.
use super::*;

#[derive(Clone, Debug)]
pub struct RowPatch {
    pub id: RowId,
    pub expected_version: String,
    pub version: String,
    pub cells: BTreeMap<ColumnId, String>,
}

impl RowPatch {
    fn checked_bytes(&self) -> Result<usize, String> {
        if self.expected_version.is_empty()
            || self.version.is_empty()
            || self.expected_version == self.version
            || self.cells.is_empty()
            || self.cells.len() > 256
        {
            return Err("Invalid row patch".into());
        }
        let mut bytes = 64 + self.cells.len() * 64;
        for value in self
            .cells
            .values()
            .chain([&self.expected_version, &self.version])
        {
            if value.len() > 65_536 {
                return Err("Update value limit exceeded".into());
            }
            bytes += value.len();
        }
        Ok(bytes)
    }
}

/// Frame-boundary delivery buffer, not a network channel. Preserve every version
/// transition; never replace an older partial patch with a newer partial patch.
/// Overflow/gaps latch an error until the host reloads and creates a new queue.
pub struct UpdateQueue {
    generation: u64,
    pending: Vec<RowPatch>,
    bytes: usize,
    error: Option<String>,
}

impl UpdateQueue {
    pub fn new(generation: u64) -> Self {
        Self {
            generation,
            pending: Vec::new(),
            bytes: 0,
            error: None,
        }
    }
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
    pub fn pending_bytes(&self) -> usize {
        self.bytes
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// A stale producer is rejected without poisoning the current subscription.
    /// All other errors require resynchronization; do not acknowledge that event.
    pub fn push(&mut self, generation: u64, patch: RowPatch) -> Result<(), String> {
        if generation != self.generation {
            return Err("Stale update generation".into());
        }
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        let bytes = match patch.checked_bytes() {
            Ok(bytes) => bytes,
            Err(error) => {
                self.error = Some(error.clone());
                return Err(error);
            }
        };
        if self.pending.len() >= 1000 || self.bytes + bytes > 1024 * 1024 {
            let error = "Update queue overflow; refresh required".to_string();
            self.error = Some(error.clone());
            return Err(error);
        }
        self.bytes += bytes;
        self.pending.push(patch);
        Ok(())
    }

    /// Call at most once per frame. Failed batches remain available for diagnosis
    /// but are never retried automatically, including after a query switch.
    pub fn flush(
        &mut self,
        pages: &mut RemotePages,
        membership_and_order_unchanged: bool,
    ) -> Result<UpdateReceipt, String> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        if self.pending.is_empty() {
            return Ok(UpdateReceipt {
                changed_rows: Vec::new(),
                cancelled_tickets: Vec::new(),
            });
        }
        match pages.apply_patches(
            self.generation,
            membership_and_order_unchanged,
            &self.pending,
        ) {
            Ok(receipt) => {
                self.pending.clear();
                self.bytes = 0;
                Ok(receipt)
            }
            Err(error) => {
                self.error = Some(error.clone());
                Err(error)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct UpdateReceipt {
    pub changed_rows: Vec<RowId>,
    /// Abort these requests: their replies could contain pre-update data.
    pub cancelled_tickets: Vec<u64>,
}

impl RemotePages {
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Host must confirm that membership/order are unchanged (including hidden
    /// business predicates). Inserts, deletes and query-affecting updates require
    /// invalidate/requery. Never call this for unconfirmed local edits.
    ///
    /// At most 1000 patches / 1 MiB per call. An error leaves cache and requests
    /// untouched; stop the stream and resynchronize after a version gap. Unknown
    /// rows are rejected, not silently dropped. Frozen pages cannot be patched.
    pub fn apply_patches(
        &mut self,
        generation: u64,
        membership_and_order_unchanged: bool,
        patches: &[RowPatch],
    ) -> Result<UpdateReceipt, String> {
        if generation != self.generation || self.query.is_none() {
            return Err("Stale update generation".into());
        }
        if !membership_and_order_unchanged {
            return Err("Query refresh required".into());
        }
        if patches.is_empty() || patches.len() > 1000 {
            return Err("Update batch must contain 1–1000 patches".into());
        }
        if self.snapshot.as_ref().is_some_and(Option::is_some) {
            return Err("Frozen pages cannot receive live updates".into());
        }
        let mut bytes = 0usize;
        for patch in patches {
            bytes = bytes.saturating_add(patch.checked_bytes()?);
            if bytes > 1024 * 1024 {
                return Err("Update batch exceeds byte budget".into());
            }
        }
        // A row can occur in overlapping cached blocks. Index all copies once;
        // validate every copy and stage only changed rows, never the whole table.
        let mut locations: BTreeMap<RowId, Vec<(usize, usize)>> = BTreeMap::new();
        for (page_index, page) in self.cache.iter().enumerate() {
            for (row_index, row) in page.page.rows.iter().enumerate() {
                locations
                    .entry(row.id)
                    .or_default()
                    .push((page_index, row_index));
            }
        }
        let mut staged: BTreeMap<(usize, usize), RemoteRow> = BTreeMap::new();
        let mut changed = BTreeSet::new();
        for patch in patches {
            let copies = locations.get(&patch.id).ok_or("Update row is not cached")?;
            for &(page_index, row_index) in copies {
                let row = staged
                    .entry((page_index, row_index))
                    .or_insert_with(|| self.cache[page_index].page.rows[row_index].clone());
                if row.version.as_deref() != Some(patch.expected_version.as_str()) {
                    return Err("Update version gap; refresh required".into());
                }
                for (column, value) in &patch.cells {
                    let cell = row.cells.get_mut(column).ok_or("Unknown update column")?;
                    *cell = value.clone();
                }
                row.version = Some(patch.version.clone());
            }
            changed.insert(patch.id);
        }
        let mut page_bytes: Vec<_> = self.cache.iter().map(|page| page.bytes).collect();
        for (&(page_index, row_index), row) in &staged {
            let old = &self.cache[page_index].page.rows[row_index];
            let text_bytes = |row: &RemoteRow| {
                row.cells
                    .values()
                    .chain(row.version.iter())
                    .map(String::len)
                    .sum::<usize>()
            };
            page_bytes[page_index] = page_bytes[page_index] - text_bytes(old) + text_bytes(row);
        }
        if page_bytes.iter().sum::<usize>() > Self::MAX_BYTES {
            return Err("Updated cache exceeds byte budget".into());
        }
        for ((page_index, row_index), row) in staged {
            self.cache[page_index].page.rows[row_index] = row;
        }
        for (page, bytes) in self.cache.iter_mut().zip(page_bytes) {
            page.bytes = bytes;
        }
        let cancelled_tickets = self.pending.keys().copied().collect();
        self.pending.clear();
        Ok(UpdateReceipt {
            changed_rows: changed.into_iter().collect(),
            cancelled_tickets,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (RemotePages, PageAddress) {
        let mut pages = RemotePages::default();
        pages.bind(RemoteQuery {
            scope: DataScope {
                source: "updates-test".into(),
                schema: 1,
                authorization: "qa".into(),
            },
            expression: None,
            filters: BTreeMap::new(),
            sorts: Vec::new(),
            search: None,
        });
        let address = PageAddress::Offset { start: 0, limit: 1 };
        let PageStart::Fetch(request) = pages.start(address.clone()).unwrap() else {
            panic!("Expected request");
        };
        assert_eq!(
            pages.accept(
                &request,
                Ok(RemotePage {
                    rows: vec![RemoteRow {
                        id: RowId(1),
                        version: Some("v1".into()),
                        cells: BTreeMap::from([(ColumnId(1), "old".into())])
                    }],
                    total: RowCount::Exact(2),
                    next_cursor: None,
                    snapshot: None,
                })
            ),
            PageAcceptance::Stored
        );
        (pages, address)
    }
    fn patch(before: &str, after: &str) -> RowPatch {
        RowPatch {
            id: RowId(1),
            expected_version: before.into(),
            version: after.into(),
            cells: BTreeMap::from([(ColumnId(1), after.into())]),
        }
    }
    #[test]
    fn patch_chain_cancels_old_reads_and_rejects_replay() {
        let (mut pages, address) = setup();
        let PageStart::Fetch(pending) = pages
            .start(PageAddress::Offset { start: 1, limit: 1 })
            .unwrap()
        else {
            panic!("Expected request")
        };
        let receipt = pages
            .apply_patches(
                pages.generation(),
                true,
                &[patch("v1", "v2"), patch("v2", "v3")],
            )
            .unwrap();
        assert_eq!(receipt.changed_rows, [RowId(1)]);
        assert_eq!(receipt.cancelled_tickets, [pending.ticket]);
        assert_eq!(
            pages.accept(&pending, Err("late".into())),
            PageAcceptance::Stale
        );
        assert!(
            pages
                .apply_patches(pages.generation(), true, &[patch("v1", "v2")])
                .is_err()
        );
        assert_eq!(
            pages.page(&address).unwrap().rows[0].version.as_deref(),
            Some("v3")
        );
    }
    #[test]
    fn patch_failure_is_atomic_and_old_scope_cannot_write() {
        let (mut pages, address) = setup();
        assert!(
            pages
                .apply_patches(
                    pages.generation(),
                    true,
                    &[patch("v1", "v2"), patch("missing", "v3")]
                )
                .is_err()
        );
        assert_eq!(
            pages.page(&address).unwrap().rows[0].cells[&ColumnId(1)],
            "old"
        );
        assert!(
            pages
                .apply_patches(pages.generation(), false, &[patch("v1", "v2")])
                .is_err()
        );
        let old = pages.generation();
        pages.invalidate();
        assert!(
            pages
                .apply_patches(old, true, &[patch("v1", "v2")])
                .is_err()
        );
    }

    #[test]
    fn patch_updates_overlapping_copies_and_rejects_frozen_or_oversized_batches() {
        let (mut pages, address) = setup();
        let overlap = PageAddress::Cursor {
            after: None,
            limit: 1,
        };
        let PageStart::Fetch(request) = pages.start(overlap.clone()).unwrap() else {
            panic!("Expected request");
        };
        let response = pages.page(&address).unwrap().clone();
        assert_eq!(pages.accept(&request, Ok(response)), PageAcceptance::Stored);
        pages
            .apply_patches(pages.generation(), true, &[patch("v1", "v2")])
            .unwrap();
        for address in [&address, &overlap] {
            assert_eq!(
                pages.page(address).unwrap().rows[0].version.as_deref(),
                Some("v2")
            );
        }
        let mut large = patch("v2", "v3");
        large.cells = (0..20)
            .map(|id| (ColumnId(id), "x".repeat(65_536)))
            .collect();
        assert!(
            pages
                .apply_patches(pages.generation(), true, &[large])
                .is_err()
        );
        pages.snapshot = Some(Some("frozen".into()));
        assert!(
            pages
                .apply_patches(pages.generation(), true, &[patch("v2", "v3")])
                .is_err()
        );
        assert_eq!(
            pages.page(&address).unwrap().rows[0].version.as_deref(),
            Some("v2")
        );
    }

    #[test]
    fn queue_preserves_field_changes_and_flushes_once() {
        let (mut pages, address) = setup();
        pages.cache[0].page.rows[0]
            .cells
            .insert(ColumnId(2), "unchanged".into());
        // Keep accounting consistent with the fixture change.
        pages.cache[0].bytes = pages.cache[0].page.validate(&address).unwrap();
        let mut queue = UpdateQueue::new(pages.generation());
        queue.push(pages.generation(), patch("v1", "v2")).unwrap();
        let mut next = patch("v2", "v3");
        next.cells = BTreeMap::from([(ColumnId(2), "second".into())]);
        queue.push(pages.generation(), next).unwrap();
        assert_eq!(queue.pending_count(), 2);
        assert!(queue.pending_bytes() > 0);
        queue.flush(&mut pages, true).unwrap();
        let row = &pages.page(&address).unwrap().rows[0];
        assert_eq!(row.cells[&ColumnId(1)], "v2");
        assert_eq!(row.cells[&ColumnId(2)], "second");
        assert_eq!(row.version.as_deref(), Some("v3"));
        assert_eq!(queue.pending_count(), 0);
        assert_eq!(queue.pending_bytes(), 0);
        assert!(
            queue
                .flush(&mut pages, true)
                .unwrap()
                .changed_rows
                .is_empty()
        );
    }

    #[test]
    fn queue_overflow_and_gap_latch_without_silent_partial_apply() {
        let (mut pages, address) = setup();
        let generation = pages.generation();
        let mut queue = UpdateQueue::new(generation);
        assert!(queue.push(generation + 1, patch("v1", "v2")).is_err());
        assert!(queue.error().is_none());
        for i in 0..1000 {
            queue
                .push(generation, patch(&format!("v{i}"), &format!("v{}", i + 1)))
                .unwrap();
        }
        assert!(queue.push(generation, patch("v1000", "v1001")).is_err());
        assert_eq!(queue.pending_count(), 1000);
        assert!(queue.flush(&mut pages, true).is_err());
        assert_eq!(
            pages.page(&address).unwrap().rows[0].version.as_deref(),
            Some("v1")
        );
        let mut queue = UpdateQueue::new(generation);
        queue.push(generation, patch("v1", "v2")).unwrap();
        queue.push(generation, patch("missing", "v3")).unwrap();
        assert!(queue.flush(&mut pages, true).is_err());
        assert!(queue.error().is_some());
        assert_eq!(queue.pending_count(), 2);
        assert_eq!(
            pages.page(&address).unwrap().rows[0].version.as_deref(),
            Some("v1")
        );
    }
}
