//! Local fixture persistence only; no business API or durable storage acceptance.
use super::*;

impl ExamplePage {
    pub(super) fn poll_durable(&mut self, ctx: &egui::Context, now: f64) {
        let (send, restored) = self.persistence.poll(ctx, &mut self.detail);
        if restored
            && let Some(id) = self
                .detail
                .bound_record()
                .and_then(|id| id.parse::<u128>().ok())
                .map(RowId)
        {
            self.selected = Some(id);
            self.main.state.row_selection.active = Some(id);
            self.child_parent = None;
            self.sync_children();
        }
        if let Some(request) = send {
            self.save_job = Some((request, now + 0.75));
        }
    }
    pub(super) fn fields(&self) -> Vec<DetailField> {
        self.selected
            .map(|id| self.fields_for(id))
            .unwrap_or_default()
    }
    pub(super) fn fields_for(&self, id: RowId) -> Vec<DetailField> {
        self.main
            .rows
            .iter()
            .find(|row| row.id == id)
            .map(|row| {
                self.main
                    .columns
                    .iter()
                    .map(|column| DetailField {
                        id: column.id.0.to_string(),
                        label: column.title.clone(),
                        value: row.text(column.id),
                        editable: matches!(column.id, NAME | NOTE),
                        required: column.id == NAME,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    pub(super) fn version(&self, id: RowId) -> String {
        self.record_versions
            .get(&id)
            .copied()
            .unwrap_or(0)
            .to_string()
    }
    pub(super) fn poll_save(&mut self, now: f64) {
        if self
            .save_job
            .as_ref()
            .is_some_and(|(_, deadline)| now >= *deadline)
            && let Some((request, _)) = self.save_job.take()
        {
            self.save(request);
        }
    }
    pub(super) fn save(&mut self, request: DetailSaveRequest) {
        if self.detail.pending_request() != Some(&request)
            || self.detail.save_phase() != Some(crate::workbench::detail::DetailSavePhase::Awaiting)
        {
            return;
        }
        let outcome = self.apply_local_save(&request);
        self.last_save_result = Some((request.clone(), outcome.clone()));
        if let Err(error) = self.detail.resolve_save(&request, outcome) {
            self.notice = Some(error);
        } else if !self.detail.has_unresolved_save() {
            self.persistence.terminal_changed();
        }
    }
    fn apply_local_save(&mut self, request: &DetailSaveRequest) -> DetailSaveOutcome {
        if let Some((original, outcome)) = &self.last_save_result
            && original == request
        {
            return outcome.clone();
        }
        let reject = |code: &str, message: &str| DetailSaveOutcome::Rejected {
            code: code.into(),
            message: message.into(),
        };
        let Ok(id) = request.record_id.parse::<u128>().map(RowId) else {
            return reject("INVALID_RECORD", "记录身份无效");
        };
        if request.observed_version != self.version(id) {
            return reject("VERSION_CONFLICT", "本地记录版本已变化");
        }
        let Some(row) = self.main.rows.iter().find(|row| row.id == id) else {
            return reject("NOT_FOUND", "当前记录不存在");
        };
        for (field, value) in &request.changes {
            let Ok(column) = field.parse::<u64>().map(ColumnId) else {
                return reject("INVALID_FIELD", "字段身份无效");
            };
            if !matches!(column, NAME | NOTE) || (column == NAME && value.trim().is_empty()) {
                return reject("READ_ONLY", "字段不可写或名称为空");
            }
            if request.before.get(field) != Some(&row.text(column)) {
                return reject("VERSION_CONFLICT", "原字段值已变化");
            }
        }
        if self.saved_logs.get(&id).map_or(0, Vec::len) >= 128 {
            return reject("LOCAL_HISTORY_LIMIT", "本地示例历史已达128项，原修改保留");
        }
        let Some(next_version) = self
            .record_versions
            .get(&id)
            .copied()
            .unwrap_or(0)
            .checked_add(1)
        else {
            return reject("VERSION_LIMIT", "本地版本已达上限");
        };
        let Some(row) = self.main.rows.iter_mut().find(|row| row.id == id) else {
            return reject("NOT_FOUND", "当前记录不存在");
        };
        for (field, value) in &request.changes {
            if let Ok(column) = field.parse::<u64>() {
                row.cells.insert(ColumnId(column), value.clone());
            }
        }
        self.record_versions.insert(id, next_version);
        let values = self
            .fields_for(id)
            .into_iter()
            .map(|field| (field.id, field.value))
            .collect();
        self.main.invalidate();
        self.notice = Some("已保存到本地示例".into());
        let mut logs = self
            .saved_logs
            .get(&id)
            .cloned()
            .unwrap_or_else(|| self.logs.rows.clone());
        logs.push(Row {
            id: RowId(id.0 * 100 + logs.len() as u128 + 1),
            cells: [
                (NUMBER, (logs.len() + 1).to_string()),
                (NAME, "修改详情".into()),
                (NOTE, "已保存本地字段".into()),
            ]
            .into_iter()
            .collect(),
        });
        self.saved_logs.insert(id, logs.clone());
        if self.selected == Some(id) {
            self.logs.replace(logs);
        }
        DetailSaveOutcome::Committed {
            version: next_version.to_string(),
            values,
        }
    }
    pub(super) fn lookup_save(&mut self, request: DetailSaveRequest) {
        let outcome = self
            .last_save_result
            .as_ref()
            .filter(|(original, _)| original == &request)
            .map(|(_, outcome)| outcome.clone())
            .unwrap_or_else(|| DetailSaveOutcome::Unknown("本地原结果尚未核实，原身份保留".into()));
        if let Err(error) = self.detail.resolve_save(&request, outcome) {
            self.notice = Some(error);
        } else if !self.detail.has_unresolved_save() {
            self.persistence.terminal_changed();
        }
    }
}

pub(super) fn can_discard_close(
    pages: &BTreeMap<String, ExamplePage>,
    page: Option<&str>,
    window: bool,
) -> bool {
    if window {
        pages
            .values()
            .all(|page| !page.detail.has_unresolved_save() && !page.persistence.pending())
    } else {
        page.and_then(|id| pages.get(id))
            .is_none_or(|page| !page.detail.has_unresolved_save() && !page.persistence.pending())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn prepare(page: &mut ExamplePage) -> DetailSaveRequest {
        page.selected = Some(RowId(1));
        page.sync_children();
        let fields = page.fields();
        page.detail.bind("1", "0", &fields);
        page.detail
            .edit_value(&NAME.0.to_string(), "本地修改".into())
            .expect("edit");
        let request = page.detail.begin_save().expect("original intent");
        let ctx = egui::Context::default();
        let root = std::env::current_dir()
            .expect("cwd")
            .join("qa/acceptance/detail-durable-native")
            .join(uuid::Uuid::new_v4().to_string());
        let loaded = crate::workbench::detail_storage::wait_storage(
            crate::workbench::detail_storage::DetailStore::open_at(
                root,
                request.scope.clone(),
                &ctx,
            )
            .expect("open"),
        )
        .expect("load");
        page.persistence =
            crate::workbench::detail_persistence::DetailPersistence::from_loaded(loaded);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            page.poll_durable(&ctx, 0.0);
            if page.save_job.is_some() {
                page.save_job = None;
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        request
    }
    fn settle_terminal(page: &mut ExamplePage) {
        let ctx = egui::Context::default();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while page.persistence.pending() {
            page.poll_durable(&ctx, 2.0);
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    #[test]
    fn detail_save_local_pending_close_blocked_and_job_finishes_once() {
        let mut page = ExamplePage::new(false);
        let request = prepare(&mut page);
        page.save_job = Some((request.clone(), 0.75));
        let mut pages: BTreeMap<_, _> = [
            ("orders".into(), page),
            ("stock".into(), ExamplePage::new(true)),
        ]
        .into_iter()
        .collect();
        assert!(!can_discard_close(&pages, Some("orders"), false));
        assert!(!can_discard_close(&pages, None, true));
        assert!(can_discard_close(&pages, Some("stock"), false));
        let page = pages.get_mut("orders").expect("page");
        page.poll_save(0.5);
        assert_eq!(page.version(RowId(1)), "0");
        page.poll_save(0.75);
        assert_eq!(page.version(RowId(1)), "1");
        let logs = page.saved_logs.get(&RowId(1)).expect("log").len();
        page.poll_save(1.5);
        assert_eq!(page.version(RowId(1)), "1");
        assert_eq!(page.saved_logs.get(&RowId(1)).expect("log").len(), logs);
        assert!(!can_discard_close(&pages, Some("orders"), false));
        settle_terminal(pages.get_mut("orders").expect("page"));
        assert!(can_discard_close(&pages, Some("orders"), false));
    }
    #[test]
    fn detail_save_local_commit_before_lost_reply_recovers_without_second_write() {
        let mut page = ExamplePage::new(false);
        let request = prepare(&mut page);
        let bytes = page
            .detail
            .checkpoint()
            .expect("checkpoint")
            .expect("pending");
        page.save(request.clone());
        let version = page.version(RowId(1));
        let logs = page.saved_logs.get(&RowId(1)).expect("logs").len();
        page.detail = DetailPanel::restore(&request.scope, &bytes).expect("restore original");
        let fields = page.fields();
        page.detail.bind("1", &version, &fields);
        let read = page.detail.begin_read().expect("original lookup");
        assert_eq!(read, request);
        page.lookup_save(read);
        assert!(!page.detail.has_unresolved_save());
        assert!(page.persistence.pending());
        settle_terminal(&mut page);
        assert!(!page.persistence.pending());
        assert!(!page.detail.is_dirty());
        assert_eq!(page.version(RowId(1)), version);
        assert_eq!(page.saved_logs.get(&RowId(1)).expect("logs").len(), logs);
        assert_eq!(page.main.rows[0].text(NAME), "本地修改");
    }
    #[test]
    fn detail_save_local_version_conflict_preserves_input_and_requires_adoption() {
        let mut page = ExamplePage::new(false);
        let request = prepare(&mut page);
        page.record_versions.insert(RowId(1), 1);
        page.main.rows[0].cells.insert(NAME, "其他修改".into());
        page.save(request.clone());
        assert_eq!(page.main.rows[0].text(NAME), "其他修改");
        assert!(page.detail.begin_save().is_err());
        let fields = page.fields();
        page.detail.bind("1", "1", &fields);
        page.detail.adopt_latest().expect("adopt actual latest");
        let next = page.detail.begin_save().expect("fresh intent");
        assert_ne!(next.operation_id, request.operation_id);
        settle_terminal(&mut page);
        let ctx = egui::Context::default();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            page.poll_durable(&ctx, 2.0);
            if page.save_job.is_some() {
                page.save_job = None;
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        page.save(next);
        assert_eq!(page.version(RowId(1)), "2");
        assert_eq!(page.main.rows[0].text(NAME), "本地修改");
    }
    #[test]
    fn detail_durable_unknown_lookup_does_not_rewrite_checkpoint() {
        let mut page = ExamplePage::new(false);
        let request = prepare(&mut page);
        page.detail
            .resolve_save(&request, DetailSaveOutcome::Unknown("unknown".into()))
            .expect("unknown");
        let read = page.detail.begin_read().expect("original read");
        page.lookup_save(read);
        assert!(page.detail.has_unresolved_save());
        assert!(!page.persistence.pending());
        assert_eq!(page.detail.pending_request(), Some(&request));
        assert_eq!(page.version(RowId(1)), "0");
    }
}
