use super::*;
use crate::named_views::NamedViewsUi;
use eframe::{Storage, egui};
use erp_grid_core::{ColumnPreference, FilterDraft, NamedViewScope, ViewPreferences};
use std::collections::BTreeMap;

const PAGE: &str = "product-skus-schema1";
const INSTANCE: &str = "bottom";

#[derive(Clone, Debug, PartialEq, Eq)]
struct Identity {
    account: super::super::views::AccountIdentity,
    parent: RowId,
}

impl Identity {
    fn scope(&self) -> NamedViewScope {
        self.account.scope(
            PAGE,
            &format!("{INSTANCE}:parent:{}", self.parent.to_uuid()),
        )
    }
}

#[derive(Default)]
pub(super) struct SkuViews {
    identity: Option<Identity>,
    named: Option<NamedViewsUi>,
    open: bool,
    error: Option<String>,
    storage_available: bool,
}

impl SkuViews {
    fn sync(
        &mut self,
        storage: &mut dyn Storage,
        account: Option<(&str, RowId, RowId)>,
        parent: Option<RowId>,
    ) {
        self.storage_available = true;
        let next = account
            .zip(parent)
            .map(|((server, tenant, actor), parent)| Identity {
                account: super::super::views::AccountIdentity::new(server, tenant, actor),
                parent,
            });
        if self.identity == next {
            if self.named.is_some() {
                self.error = None;
            }
            return;
        }
        if let Some(named) = &mut self.named
            && let Err(error) = named.persist(storage)
        {
            self.storage_available = false;
            self.open = false;
            self.error = Some(format!("SKU views could not be stored: {error}"));
            return;
        }
        self.identity = next;
        self.named = None;
        self.open = false;
        self.error = None;
        let Some(identity) = &self.identity else {
            return;
        };
        match NamedViewsUi::load(storage, identity.scope()) {
            Ok(named) => self.named = Some(named),
            Err(error) => self.error = Some(format!("SKU views unavailable: {error}")),
        }
    }

    fn set_unavailable(&mut self) {
        self.storage_available = false;
        self.open = false;
        self.error = Some("SKU views require local storage".into());
    }

    fn scope(&self) -> Option<String> {
        self.storage_available
            .then(|| {
                self.identity
                    .as_ref()
                    .map(|identity| identity.scope().preference_scope())
            })
            .flatten()
    }

    fn persist(&mut self, storage: &mut dyn Storage) -> Result<(), String> {
        if let Some(named) = &mut self.named {
            named.persist(storage)?;
        }
        Ok(())
    }
}

impl Details {
    pub(in crate::remote_view) fn sync_views(
        &mut self,
        storage: &mut dyn Storage,
        account: Option<(&str, RowId, RowId)>,
    ) {
        self.views.sync(storage, account, self.draft_parent());
    }

    pub(in crate::remote_view) fn views_storage_unavailable(&mut self) {
        self.views.set_unavailable();
    }

    pub(in crate::remote_view) fn persist_views(
        &mut self,
        storage: &mut dyn Storage,
    ) -> Result<(), String> {
        self.views.persist(storage)
    }

    fn current_view(&self) -> Option<ViewPreferences> {
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
            page_size: self.pager.page_size(),
            quick_search_open: self.state.quick_search_open,
            search: (!self.search.trim().is_empty()).then(|| self.search.trim().to_owned()),
            expression: None,
        })
    }

    fn apply_view(
        &mut self,
        preferences: ViewPreferences,
        session: &Session,
        parent: RowId,
    ) -> Result<(), String> {
        if self.dirty() {
            return Err("请先完成当前 SKU 编辑".into());
        }
        let scope = self.views.scope().ok_or("SKU 账号视图尚未就绪")?;
        if ![10, 20, 50, 100, 200].contains(&preferences.page_size) {
            return Err("不支持的 SKU 每页行数".into());
        }
        if preferences.expression.is_some() {
            return Err("SKU 视图不支持组合条件".into());
        }
        let mut columns = self.columns.clone();
        preferences
            .apply_columns(&scope, &mut columns)
            .map_err(str::to_owned)?;
        let mut drafts = BTreeMap::new();
        let mut filters = BTreeMap::new();
        for (id, draft) in &preferences.filters {
            let column = columns
                .iter()
                .find(|column| column.id == *id)
                .ok_or("SKU 视图包含未知列")?;
            let valid = matches!(
                (id.0, draft),
                (0 | 1 | 2 | 3 | 6, FilterDraft::Text(_))
                    | (4, FilterDraft::Money { .. })
                    | (5, FilterDraft::Enum(_))
                    | (7 | 8, FilterDraft::DateTime { .. })
            );
            if !valid {
                return Err("SKU 视图筛选类型与字段不匹配".into());
            }
            if let Some(filter) = draft.compile(&column.filter).map_err(str::to_owned)? {
                filters.insert(*id, filter);
            }
            if drafts.insert(*id, draft.clone()).is_some() {
                return Err("SKU 视图包含重复筛选列".into());
            }
        }
        let query = RemoteQuery {
            expression: None,
            scope: DataScope {
                source: format!(
                    "product-skus:{}:{}",
                    session.tenant.to_uuid(),
                    parent.to_uuid()
                ),
                schema: 1,
                authorization: format!("session:{}", self.generation),
            },
            filters,
            sorts: preferences.sorts.clone(),
            search: preferences.search.clone(),
        };
        encode_page_query(&PageRequest {
            ticket: 0,
            generation: self.generation,
            query,
            address: PageAddress::Offset {
                start: 0,
                limit: preferences.page_size as u16,
            },
        })?;
        self.columns = columns;
        self.state.quick_search = drafts;
        self.state.quick_search_open = preferences.quick_search_open;
        self.state.sorts = preferences.sorts;
        self.search = preferences.search.unwrap_or_default();
        self.pager.set_page_size(preferences.page_size);
        self.reset(true);
        Ok(())
    }

    pub(super) fn views_ui(&mut self, ui: &mut egui::Ui, session: &Session, parent: Option<RowId>) {
        let current = self.current_view();
        let available = current.is_some() && parent.is_some();
        let clean = !self.dirty();
        if ui
            .add_enabled(available && clean, egui::Button::new("SKU Views"))
            .clicked()
        {
            self.views.open = true;
        }
        let mut applied = None;
        if self.views.open && available {
            egui::Window::new("SKU Views")
                .id(ui.id().with("remote-sku-views"))
                .open(&mut self.views.open)
                .show(ui.ctx(), |ui| {
                    ui.add_enabled_ui(clean, |ui| {
                        if let (Some(named), Some(current)) = (&mut self.views.named, &current) {
                            applied = named.ui(ui, current);
                        }
                    });
                });
        }
        if let Some(error) = &self.views.error {
            ui.colored_label(egui::Color32::YELLOW, "!")
                .on_hover_text(error);
        }
        if let (Some(preferences), Some(parent)) = (applied, parent)
            && let Err(error) = self.apply_view(preferences, session, parent)
        {
            self.error = Some(error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::named_views::storage_key;

    #[derive(Default)]
    struct MemoryStorage(BTreeMap<String, String>);
    impl Storage for MemoryStorage {
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

    fn session() -> Session {
        Session {
            client: HttpClient::new("http://127.0.0.1:1/").unwrap(),
            tenant: RowId(1),
            token: "fixture".into(),
            expires: web_time::Instant::now() + Duration::from_secs(60),
        }
    }

    #[test]
    fn sku_view_scope_includes_verified_account_and_parent() {
        let mut storage = MemoryStorage::default();
        let mut details = Details {
            parent: Some((7, RowId(10))),
            ..Default::default()
        };
        details.sync_views(
            &mut storage,
            Some(("https://erp.example", RowId(1), RowId(2))),
        );
        let first = details.views.identity.as_ref().unwrap().scope();
        details.parent = Some((8, RowId(11)));
        details.sync_views(
            &mut storage,
            Some(("https://erp.example", RowId(1), RowId(2))),
        );
        let second = details.views.identity.as_ref().unwrap().scope();
        assert_ne!(storage_key(&first).unwrap(), storage_key(&second).unwrap());
        assert_eq!(first.page, PAGE);
        assert!(first.instance.contains(&RowId(10).to_uuid().to_string()));
    }

    #[test]
    fn full_sku_query_applies_atomically_and_invalid_candidate_keeps_draft() {
        let mut storage = MemoryStorage::default();
        let mut details = Details {
            parent: Some((7, RowId(10))),
            ..Default::default()
        };
        details.sync_views(
            &mut storage,
            Some(("http://127.0.0.1:1/", RowId(1), RowId(2))),
        );
        let mut preferences = details.current_view().unwrap();
        preferences.search = Some("blue".into());
        preferences.page_size = 50;
        preferences.columns[0].width = 260.0;
        preferences.filters = vec![(
            ColumnId(4),
            FilterDraft::Money {
                min: "10.00".into(),
                max: "99.00".into(),
            },
        )];
        preferences.sorts = vec![erp_grid_core::SortRule {
            column: ColumnId(4),
            descending: true,
        }];
        details
            .apply_view(preferences, &session(), RowId(10))
            .unwrap();
        assert_eq!(details.search, "blue");
        assert_eq!(details.pager.page_size(), 50);
        assert_eq!(details.columns[0].width(), 260.0);
        let before = details.current_view().unwrap();
        let mut invalid = before.clone();
        invalid.columns[0].width = 500.0;
        invalid.filters = vec![(ColumnId(4), FilterDraft::Text("bad".into()))];
        assert!(details.apply_view(invalid, &session(), RowId(10)).is_err());
        assert_eq!(details.current_view().unwrap(), before);
    }
}
