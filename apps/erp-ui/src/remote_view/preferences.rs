use super::*;
use erp_grid_core::{ColumnPreference, FilterDraft, ViewPreferences};
use std::collections::BTreeMap;

impl RemoteProductView {
    pub fn sync_views(&mut self, storage: &mut dyn eframe::Storage) {
        let identity = self
            .session
            .as_ref()
            .filter(|session| web_time::Instant::now() < session.expires)
            .and_then(|session| {
                self.actions
                    .actor()
                    .map(|actor| (session.client.base_url(), session.tenant, actor))
            });
        self.views.sync(storage, identity);
        self.details.sync_views(storage, identity);
        self.orders.sync_views(storage, identity);
        let scope = self.page_scope();
        if self.workbench.sync(storage, scope.as_deref(), "products") {
            self.inspector.open = self.workbench.preferences.right_open;
        }
        self.orders
            .workbench
            .sync(storage, scope.as_deref(), "orders");
        self.views_storage_ready = true;
    }

    pub fn views_storage_unavailable(&mut self) {
        self.views_storage_ready = false;
        self.views.set_unavailable();
        self.details.views_storage_unavailable();
        self.orders.views_storage_unavailable();
    }

    pub fn persist_views(&mut self, storage: &mut dyn eframe::Storage) {
        if let Err(error) = self.views.persist(storage) {
            self.notice = Some(format!("视图保存失败：{error}"));
        }
        if let Err(error) = self.details.persist_views(storage) {
            self.notice = Some(format!("SKU 视图保存失败：{error}"));
        }
        if let Err(error) = self.orders.persist_views(storage) {
            self.notice = Some(format!("订单视图保存失败：{error}"));
        }
        if self.workbench.preferences.right_open != self.inspector.open {
            self.workbench.preferences.right_open = self.inspector.open;
            self.workbench.mark_dirty();
        }
        self.workbench.persist(storage, "products");
        self.orders.workbench.persist(storage, "orders");
    }

    pub(super) fn view_preferences(&self) -> Option<ViewPreferences> {
        Some(ViewPreferences {
            version: 1,
            scope: self.views.scope()?,
            columns: self
                .columns
                .iter()
                .map(|column| ColumnPreference {
                    id: column.id,
                    width: column.width(),
                    visible: column.visible,
                    pinned: column.pinned,
                    pinned_right: column.pinned_right,
                })
                .collect(),
            filters: self
                .state
                .quick_search
                .iter()
                .map(|(id, draft)| (*id, draft.clone()))
                .collect(),
            sorts: self.state.sorts.clone(),
            page_size: self.address.limit(),
            quick_search_open: self.state.quick_search_open,
            search: (!self.search.trim().is_empty()).then(|| self.search.trim().to_owned()),
            expression: self.conditions.compile(&self.columns).ok()?,
        })
    }

    pub(super) fn apply_view(&mut self, preferences: ViewPreferences) -> Result<(), String> {
        if self.dirty() {
            return Err("请先完成当前编辑".into());
        }
        let scope = self.views.scope().ok_or("账号视图尚未就绪")?;
        if ![10, 20, 50, 100, 200, 500].contains(&preferences.page_size) {
            return Err("不支持的每页行数".into());
        }
        let mut columns = self.columns.clone();
        preferences
            .apply_columns(&scope, &mut columns)
            .map_err(str::to_owned)?;
        let mut drafts = BTreeMap::new();
        let mut filters = BTreeMap::new();
        for (id, draft) in &preferences.filters {
            let column = columns
                .iter_mut()
                .find(|column| column.id == *id)
                .ok_or("视图包含未知列")?;
            match (id.0, draft) {
                (0..=4, FilterDraft::Text(_)) => column.filter = FilterKind::Text,
                (0..=4, FilterDraft::Enum(values)) => {
                    column.filter = FilterKind::Enum(values.iter().cloned().collect())
                }
                (5..=6, FilterDraft::DateTime { .. }) => column.filter = FilterKind::DateTime,
                _ => return Err("视图筛选类型与字段不匹配".into()),
            }
            if let Some(filter) = draft.compile(&column.filter).map_err(str::to_owned)? {
                filters.insert(*id, filter);
            }
            if drafts.insert(*id, draft.clone()).is_some() {
                return Err("视图包含重复筛选列".into());
            }
        }
        if let Some(expression) = &preferences.expression {
            expression.validate().map_err(str::to_owned)?;
        }
        let conditions = erp_ui_egui::query_builder::QueryBuilder::from_expression(
            preferences.expression.as_ref(),
            &columns,
        )?;
        let session = self.session.as_ref().ok_or("账号已退出")?;
        let query = RemoteQuery {
            scope: DataScope {
                source: format!("products:{}", session.tenant.to_uuid()),
                schema: 1,
                authorization: format!("session:{}", self.session_number),
            },
            filters,
            sorts: preferences.sorts.clone(),
            search: preferences.search.clone(),
            expression: preferences.expression.clone().map(Box::new),
        };
        products::encode_query(
            session.tenant,
            &PageRequest {
                ticket: 0,
                generation: self.generation,
                query,
                address: PageAddress::Offset {
                    start: 0,
                    limit: preferences.page_size as u16,
                },
            },
        )?;
        // Validation above only touched candidates; rejected views leave the page intact.
        self.columns = columns;
        self.state.quick_search = drafts;
        self.state.quick_search_open = preferences.quick_search_open;
        self.state.sorts = preferences.sorts;
        self.search = preferences.search.unwrap_or_default();
        self.conditions = conditions;
        self.address = PageAddress::Offset {
            start: 0,
            limit: preferences.page_size as u16,
        };
        self.deadline = None;
        self.options.clear();
        self.reset_query(true);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_grid_core::{Filter, paged_query::FilterExpression};

    #[derive(Default)]
    struct MemoryStorage(BTreeMap<String, String>);
    impl eframe::Storage for MemoryStorage {
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

    fn host() -> RemoteProductView {
        let mut host = RemoteProductView {
            session: Some(Session {
                client: HttpClient::new("http://127.0.0.1:1/").unwrap(),
                tenant: RowId(1),
                token: "fixture".into(),
                expires: web_time::Instant::now() + Duration::from_secs(60),
            }),
            ..Default::default()
        };
        host.views.sync(
            &mut MemoryStorage::default(),
            Some(("http://127.0.0.1:1/", RowId(1), RowId(2))),
        );
        host
    }

    #[test]
    fn account_view_applies_full_query_and_rejects_bad_candidate_atomically() {
        let mut host = host();
        let mut preferences = host.view_preferences().unwrap();
        preferences.search = Some("needle".into());
        preferences.page_size = 50;
        preferences.columns[0].width = 230.0;
        preferences.filters = vec![(ColumnId(2), FilterDraft::Text("red".into()))];
        preferences.expression = Some(FilterExpression::Or(vec![
            FilterExpression::Condition {
                column: ColumnId(2),
                filter: Filter::Text("brand".into()),
            },
            FilterExpression::Condition {
                column: ColumnId(2),
                filter: Filter::Enum(["old-value".to_string()].into()),
            },
        ]));
        host.apply_view(preferences.clone()).unwrap();
        assert!(!host.invalid_query);
        let query = host.query.as_ref().unwrap();
        assert_eq!(query.search, preferences.search);
        assert_eq!(query.expression.as_deref(), preferences.expression.as_ref());
        assert_eq!(
            query.filters.get(&ColumnId(2)),
            Some(&Filter::Text("red".into()))
        );
        assert_eq!(host.address.limit(), 50);
        let before = host.view_preferences().unwrap();
        let mut invalid = before.clone();
        invalid.columns[0].width = 500.0;
        invalid.filters = vec![(
            ColumnId(2),
            FilterDraft::DateTime {
                start: None,
                end: None,
            },
        )];
        assert!(host.apply_view(invalid).is_err());
        assert_eq!(host.view_preferences().unwrap(), before);
    }

    #[test]
    fn account_view_signout_resets_columns_without_dropping_store_before_sync() {
        let mut host = host();
        host.columns[0].resize(400.0);
        let old_scope = host.views.scope();
        host.disconnect();
        assert_eq!(
            host.columns[0].width(),
            RemoteProductView::default().columns[0].width()
        );
        assert_eq!(host.views.scope(), old_scope);
        host.sync_views(&mut MemoryStorage::default());
        assert!(host.views.scope().is_none());
    }
}
