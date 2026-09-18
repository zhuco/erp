use super::*;
use erp_grid_core::{ColumnPreference, ViewPreferences};

impl Orders {
    pub(in crate::remote_view) fn sync_views(
        &mut self,
        storage: &mut dyn eframe::Storage,
        identity: Option<(&str, RowId, RowId)>,
    ) {
        self.views.sync(storage, identity);
        let scope = identity.map(|(server, tenant, actor)| {
            format!("{}:{}:{}", server, tenant.to_uuid(), actor.to_uuid())
        });
        if self.workbench.sync(storage, scope.as_deref(), "orders") {
            self.keyword.clear();
            self.period_days = 0;
        }
        self.sync_recovery(identity);
        self.inspector.sync_recovery_identity(identity);
    }

    pub(in crate::remote_view) fn views_storage_unavailable(&mut self) {
        self.views.set_unavailable();
    }

    pub(in crate::remote_view) fn persist_views(
        &mut self,
        storage: &mut dyn eframe::Storage,
    ) -> Result<(), String> {
        self.views.persist(storage)?;
        self.workbench.persist(storage, "orders");
        Ok(())
    }

    fn sync_recovery(&mut self, identity: Option<(&str, RowId, RowId)>) {
        let identity = identity.map(|(server, tenant, actor)| (server.to_owned(), tenant, actor));
        self.recovery_identity_verified = identity.is_some() && self.recovery_identity == identity;
        if self.recovery_identity == identity {
            return;
        }
        if self.recovery_identity.is_some()
            && (self.prepared.is_some()
                || self.pending.is_some()
                || self.unknown.is_some()
                || self.item_state.is_editing()
                || self.recovery_snapshot.is_some()
                || self.recovery_corrupt
                || self.recovery_opening.is_some()
                || self.recovery_saving.is_some())
        {
            self.notice = Some("账号身份变化，订单草稿保持隔离且禁止派发".into());
            return;
        }
        self.recovery_identity = identity.clone();
        self.recovery_identity_verified = identity.is_some();
        self.recovery_snapshot = None;
        self.recovery_corrupt = false;
        self.recovery_clear_requested = false;
        self.recovery_restored = false;
        self.recovery_opening = None;
        self.recovery_store = None;
        self.recovery_revision = 0;
        self.recovery_saving = None;
    }

    pub(super) fn tick_recovery(&mut self, session: &Session, ctx: &egui::Context) {
        if self.recovery_opening.is_some() || self.recovery_saving.is_some() {
            ctx.request_repaint();
        }
        let Some((server, tenant, actor)) = self.recovery_identity.clone() else {
            return;
        };
        if !self.recovery_account_matches(session) {
            self.notice = Some("订单恢复账号身份不匹配".into());
            return;
        }
        if self.recovery_store.is_none()
            && self.recovery_opening.is_none()
            && !self.recovery_corrupt
        {
            let scope =
                match crate::remote_actions::ActionScope::new(&session.client, tenant, actor) {
                    Ok(scope) => scope,
                    Err(error) => {
                        self.recovery_corrupt = true;
                        self.notice = Some(error);
                        return;
                    }
                };
            match OrderRecoveryStore::open(scope, ctx) {
                Ok(task) => self.recovery_opening = Some(task),
                Err(error) => {
                    self.recovery_corrupt = true;
                    self.notice = Some(error);
                }
            }
        }
        if let Some(result) = self.recovery_opening.as_mut().and_then(StorageTask::poll) {
            self.recovery_opening = None;
            match result {
                Ok(loaded) => {
                    self.recovery_revision = loaded.revision;
                    self.recovery_store = Some(loaded.store);
                    if let Some(bytes) = loaded.bytes {
                        match drafts::decode_for_account(
                            &server,
                            tenant,
                            actor,
                            super::super::unix_seconds(),
                            &bytes,
                        ) {
                            Ok(snapshot) => self.accept_recovery_snapshot(snapshot),
                            Err(error) => {
                                self.recovery_corrupt = true;
                                self.notice =
                                    Some(format!("订单恢复记录损坏，已阻止覆盖：{error}"));
                            }
                        }
                    }
                }
                Err(error) => {
                    self.recovery_corrupt = true;
                    self.notice = Some(error);
                }
            }
        }
        if let Some(result) = self
            .recovery_saving
            .as_mut()
            .and_then(|(task, _)| task.poll())
        {
            let Some((_, bytes)) = self.recovery_saving.take() else {
                return;
            };
            match result {
                Ok(revision) => {
                    self.recovery_revision = revision;
                    self.recovery_clear_requested = false;
                    self.recovery_snapshot = match bytes.as_deref() {
                        Some(bytes) => match drafts::decode_for_account(
                            &server,
                            tenant,
                            actor,
                            super::super::unix_seconds(),
                            bytes,
                        ) {
                            Ok(snapshot) => Some(snapshot),
                            Err(error) => {
                                self.recovery_corrupt = true;
                                self.notice =
                                    Some(format!("已提交的订单恢复记录无法解码：{error}"));
                                return;
                            }
                        },
                        None => None,
                    };
                    // This process already owns the edit that produced this commit.
                    // Only a snapshot loaded when opening storage needs restoration.
                    self.recovery_restored = true;
                }
                Err(error) => {
                    self.recovery_corrupt = true;
                    self.notice = Some(error);
                }
            }
        }
        if self.recovery_saving.is_some() || self.recovery_store.is_none() || self.recovery_corrupt
        {
            return;
        }
        let bytes = if self.recovery_clear_requested {
            None
        } else {
            let Some(scope) = self.recovery_scope(&server, tenant, actor) else {
                return;
            };
            let recovery = match self.current_recovery(&scope) {
                Ok(Some(recovery)) => recovery,
                Ok(None)
                    if self
                        .recovery_snapshot
                        .as_ref()
                        .is_some_and(|snapshot| !snapshot.unknown())
                        && self.recovery_restored =>
                {
                    self.recovery_clear_requested = true;
                    return;
                }
                Ok(None) => return,
                Err(error) => {
                    self.notice = Some(error);
                    return;
                }
            };
            if self
                .recovery_snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.recovery == recovery)
            {
                return;
            }
            match drafts::encode(
                &scope,
                super::super::unix_seconds(),
                self.recovery_snapshot.as_ref(),
                &recovery,
            ) {
                Ok(bytes) => Some(bytes),
                Err(error) => {
                    self.notice = Some(error);
                    return;
                }
            }
        };
        let Some(store) = self.recovery_store.as_ref() else {
            return;
        };
        match store.commit(self.recovery_revision, bytes.as_deref(), ctx) {
            Ok(task) => self.recovery_saving = Some((task, bytes)),
            Err(error) => {
                self.recovery_corrupt = true;
                self.notice = Some(error);
            }
        }
    }

    fn accept_recovery_snapshot(&mut self, snapshot: drafts::Snapshot) {
        self.recovery_restored = false;
        let expired = snapshot.expired(super::super::unix_seconds());
        self.selected = Some(snapshot.scope.order_id.clone());
        self.detail = None;
        self.item_rows.clear();
        self.detail_failed = false;
        self.detail_task = None;
        self.recovery_snapshot = Some(snapshot);
        self.notice = Some(if expired {
            "发现已过期的订单货品恢复记录，仍保留并等待人工核对".into()
        } else {
            "发现订单货品恢复记录，正在读取订单详情".into()
        });
    }

    pub(super) fn recovery_account_matches(&self, session: &Session) -> bool {
        self.recovery_identity_verified
            && self
                .recovery_identity
                .as_ref()
                .is_some_and(|(server, tenant, _)| {
                    server == session.client.base_url() && *tenant == session.tenant
                })
    }

    fn recovery_scope(&self, server: &str, tenant: RowId, actor: RowId) -> Option<drafts::Scope> {
        Some(drafts::Scope {
            server: server.to_owned(),
            tenant,
            actor,
            order_id: self.selected.clone()?,
            schema: 1,
        })
    }

    fn current_recovery(&self, scope: &drafts::Scope) -> Result<Option<drafts::Recovery>, String> {
        let request = self
            .prepared
            .as_ref()
            .map(|value| {
                (
                    &value.edit,
                    value.expected_version,
                    &value.path,
                    &value.json,
                )
            })
            .or_else(|| {
                self.pending.as_ref().map(|value| {
                    (
                        &value.edit,
                        value.expected_version,
                        &value.path,
                        &value.json,
                    )
                })
            })
            .or_else(|| {
                self.unknown.as_ref().map(|value| {
                    (
                        &value.edit,
                        value.expected_version,
                        &value.path,
                        &value.json,
                    )
                })
            });
        if let Some((edit, expected_version, path, json)) = request {
            let edit = self.draft_edit(scope, edit, expected_version)?;
            return Ok(Some(drafts::dispatching(drafts::OriginalRequest {
                path: path.clone(),
                body: json.clone(),
                expected_version,
                edit,
            })));
        }
        let Some(edit) = self.item_state.edit_draft() else {
            return Ok(None);
        };
        if edit.before == edit.after {
            return Ok(None);
        }
        let version = self
            .detail
            .as_ref()
            .ok_or("订单详情尚未加载")?
            .order
            .version;
        Ok(Some(drafts::unsent(
            self.draft_edit(scope, &edit, version)?,
        )))
    }

    fn draft_edit(
        &self,
        scope: &drafts::Scope,
        edit: &erp_grid_core::CellEdit,
        expected_version: i32,
    ) -> Result<drafts::DraftEdit, String> {
        let index = self
            .item_rows
            .iter()
            .position(|row| *row == edit.row)
            .ok_or("订单货品恢复身份已变化")?;
        let line_id = self
            .detail
            .as_ref()
            .and_then(|detail| detail.items.get(index))
            .map(|item| item.id.clone())
            .ok_or("订单货品恢复行已不存在")?;
        if stable_child_id(&scope.order_id, &line_id) != edit.row {
            return Err("订单货品恢复身份不匹配".into());
        }
        Ok(drafts::DraftEdit {
            line_id,
            row: edit.row,
            column: edit.column,
            before: edit.before.clone(),
            after: edit.after.clone(),
            expected_version,
        })
    }

    fn view_preferences(&self) -> Option<ViewPreferences> {
        let mut filters = self.order_state.quick_search.clone();
        if self.period_days > 0
            && filters
                .get(&CREATED)
                .is_none_or(|draft| matches!(draft.compile(&FilterKind::DateTime), Ok(None)))
        {
            let now = chrono::Utc::now();
            let start = now - chrono::Duration::days(i64::from(self.period_days));
            let parts = |date: chrono::DateTime<chrono::Utc>| {
                erp_grid_core::DateTimeParts::parse(&date.format("%Y-%m-%d %H:%M:%S").to_string())
                    .ok()
            };
            filters.insert(
                CREATED,
                FilterDraft::DateTime {
                    start: parts(start),
                    end: parts(now),
                },
            );
        }
        Some(ViewPreferences {
            version: 1,
            scope: self.views.scope()?,
            columns: self
                .order_columns
                .iter()
                .map(|column| ColumnPreference {
                    id: column.id,
                    width: column.width(),
                    visible: column.visible,
                    pinned: column.pinned,
                    pinned_right: column.pinned_right,
                })
                .collect(),
            filters: filters
                .iter()
                .map(|(id, draft)| (*id, draft.clone()))
                .collect(),
            sorts: Vec::new(),
            page_size: self.page_size,
            quick_search_open: self.order_state.quick_search_open,
            search: Some(self.keyword.clone()),
            expression: None,
        })
    }

    pub(super) fn views_ui(&mut self, ui: &mut egui::Ui, _session: &Session) {
        let Some(current) = self.view_preferences() else {
            return;
        };
        let ready = !self.dirty() && self.list_task.is_none() && self.search_deadline.is_none();
        if let Some(view) = self.views.ui(ui, &current, ready)
            && let Err(error) = self.apply_view(view, ui.ctx())
        {
            self.notice = Some(error);
        }
    }

    fn apply_view(&mut self, view: ViewPreferences, ctx: &egui::Context) -> Result<(), String> {
        if self.dirty() {
            return Err("请先完成当前编辑".into());
        }
        let scope = self.views.scope().ok_or("订单账号视图尚未就绪")?;
        if ![10, 25, 50, 100].contains(&view.page_size)
            || !view.sorts.is_empty()
            || view.expression.is_some()
            || view.search.as_ref().is_some_and(|value| value.len() > 4096)
        {
            return Err("订单视图包含未支持的查询设置".into());
        }
        let mut columns = self.order_columns.clone();
        view.apply_columns(&scope, &mut columns)
            .map_err(str::to_owned)?;
        let mut filters = std::collections::BTreeMap::new();
        for (id, draft) in &view.filters {
            let column = columns
                .iter()
                .find(|column| column.id == *id)
                .ok_or("视图包含未知列")?;
            draft.compile(&column.filter).map_err(str::to_owned)?;
            if filters.insert(*id, draft.clone()).is_some() {
                return Err("视图包含重复筛选列".into());
            }
        }
        amount_query(filters.get(&TOTAL))?;
        self.keyword = view.search.unwrap_or_default();
        self.period_days = 0;
        self.order_columns = columns;
        self.order_state.quick_search = filters;
        self.order_state.quick_search_open = true;
        self.page_size = view.page_size;
        self.offset = 0;
        self.list_task = None;
        self.orders.clear();
        self.order_rows.clear();
        self.total = 0;
        self.selected = None;
        self.detail_task = None;
        self.latest_task = None;
        self.detail = None;
        self.item_rows.clear();
        self.detail_generation = self.detail_generation.wrapping_add(1);
        self.order_state.bind_view(self.detail_generation);
        self.item_state.bind_view(self.detail_generation);
        self.search_deadline = Some(ctx.input(|input| input.time));
        ctx.request_repaint();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn another_actor_cannot_rebind_an_unresolved_order_request() {
        let mut orders = Orders::default();
        orders.sync_recovery(Some(("https://qa.invalid/", RowId(1), RowId(2))));
        orders.unknown = Some(UnknownEdit {
            edit: CellEdit {
                row: RowId(1),
                column: QTY,
                before: "2".into(),
                after: "3".into(),
            },
            expected_version: 1,
            path: "/original".into(),
            json: Vec::new(),
        });
        orders.sync_recovery(Some(("https://qa.invalid/", RowId(1), RowId(3))));
        assert!(!orders.recovery_identity_verified);
        assert_eq!(orders.recovery_identity.as_ref().unwrap().2, RowId(2));
        assert!(orders.unknown.is_some());
        orders.sync_recovery(Some(("https://qa.invalid/", RowId(1), RowId(2))));
        assert!(orders.recovery_identity_verified);
    }

    #[derive(Default)]
    struct Storage(std::collections::HashMap<String, String>);
    impl eframe::Storage for Storage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.into(), value);
        }
        fn remove_string(&mut self, key: &str) {
            self.0.remove(key);
        }
        fn flush(&mut self) {}
    }

    #[test]
    fn order_view_rejects_foreign_scope_and_invalid_filter_atomically() {
        let mut orders = Orders::default();
        orders.sync_views(
            &mut Storage::default(),
            Some(("https://qa.invalid", RowId(1), RowId(2))),
        );
        let baseline = orders.view_preferences().unwrap();
        let mut foreign = baseline.clone();
        foreign.scope = "products-schema1".into();
        foreign.page_size = 10;
        assert!(
            orders
                .apply_view(foreign, &egui::Context::default())
                .is_err()
        );
        assert_eq!(orders.page_size, 50);
        let mut invalid = baseline;
        invalid.filters = vec![(PLATFORM, FilterDraft::Text("jd".into()))];
        assert!(
            orders
                .apply_view(invalid, &egui::Context::default())
                .is_err()
        );
        assert!(orders.order_state.quick_search.is_empty());
        assert!(orders.search_deadline.is_none());
    }

    #[test]
    fn applying_order_view_restores_query_and_resets_page() {
        let mut orders = Orders::default();
        orders.sync_views(
            &mut Storage::default(),
            Some(("https://qa.invalid", RowId(1), RowId(2))),
        );
        orders.offset = 100;
        let mut view = orders.view_preferences().unwrap();
        view.page_size = 10;
        view.filters = vec![(ORDER_ID, FilterDraft::Text("QA-ORDER-001".into()))];
        orders.apply_view(view, &egui::Context::default()).unwrap();
        assert_eq!(orders.offset, 0);
        assert_eq!(orders.page_size, 10);
        assert_eq!(selection_filters(&orders.order_state).0, "QA-ORDER-001");
        assert!(orders.search_deadline.is_some());
    }
}
