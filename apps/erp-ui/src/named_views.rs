//! Small egui host adapter for the core named-view store.
//! The owning application supplies the account/page/instance scope and the
//! current ViewPreferences; this module never owns a TableView.

use eframe::{Storage, egui};
use erp_grid_core::{NamedViewScope, NamedViewStore, ViewPreferences};

const MAX_NAMED_VIEW_BYTES: usize = 8 * 1024 * 1024;

pub struct NamedViewsUi {
    store: NamedViewStore,
    selected: Option<String>,
    name: String,
    rename: String,
    dirty: bool,
    pub notice: Option<String>,
}

impl NamedViewsUi {
    pub fn new(scope: NamedViewScope) -> Result<Self, String> {
        Ok(Self {
            store: NamedViewStore::new(scope)?,
            selected: None,
            name: String::new(),
            rename: String::new(),
            dirty: false,
            notice: None,
        })
    }

    pub fn load(storage: &dyn Storage, scope: NamedViewScope) -> Result<Self, String> {
        let key = storage_key(&scope)?;
        let store = match storage.get_string(&key) {
            Some(text) => {
                if text.len() > MAX_NAMED_VIEW_BYTES {
                    return Err("Named-view data exceeds storage limit".into());
                }
                let candidate: NamedViewStore = serde_json::from_str(&text)
                    .map_err(|_| "Invalid named-view data".to_owned())?;
                candidate.validate(&scope)?;
                candidate
            }
            None => NamedViewStore::new(scope)?,
        };
        Ok(Self {
            store,
            selected: None,
            name: String::new(),
            rename: String::new(),
            dirty: false,
            notice: None,
        })
    }

    pub fn persist(&mut self, storage: &mut dyn Storage) -> Result<(), String> {
        if !self.dirty {
            return Ok(());
        }
        let key = storage_key(&self.store.scope)?;
        self.store.validate(&self.store.scope)?;
        let bytes = serde_json::to_vec(&self.store)
            .map_err(|_| "Unable to encode named views".to_owned())?;
        if bytes.len() > MAX_NAMED_VIEW_BYTES {
            return Err("Named-view data exceeds storage limit".into());
        }
        let text =
            String::from_utf8(bytes).map_err(|_| "Invalid named-view encoding".to_owned())?;
        storage.set_string(&key, text);
        self.dirty = false;
        Ok(())
    }

    pub fn selected_preferences(&self) -> Option<&ViewPreferences> {
        self.selected
            .as_deref()
            .and_then(|name| self.store.get(name))
    }

    /// Returns a cloned view when the user presses Apply. The caller should
    /// replace its current preferences and persist the store after a change.
    pub fn ui(&mut self, ui: &mut egui::Ui, current: &ViewPreferences) -> Option<ViewPreferences> {
        let mut applied = None;
        ui.horizontal(|ui| {
            ui.label("Views");
            ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(180.0));
            if ui.button("Save").clicked() {
                self.commit_save(current.clone(), false);
            }
            if ui.button("Overwrite").clicked() {
                self.commit_save(current.clone(), true);
            }
        });
        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(ui, |ui| {
                for view in self.store.list() {
                    let selected = self.selected.as_deref() == Some(view.name.as_str());
                    if ui.selectable_label(selected, &view.name).clicked() {
                        self.selected = Some(view.name.clone());
                        self.rename = view.name.clone();
                    }
                }
            });
        ui.horizontal(|ui| {
            if ui.button("Apply").clicked() {
                applied = self.selected_preferences().cloned();
            }
            ui.add(egui::TextEdit::singleline(&mut self.rename).desired_width(180.0));
            if ui.button("Rename").clicked()
                && let Some(old) = self.selected.clone()
            {
                match self.store.rename(&old, self.rename.clone()) {
                    Ok(()) => {
                        self.selected = Some(self.rename.clone());
                        self.dirty = true;
                    }
                    Err(error) => self.notice = Some(error),
                }
            }
            if ui.button("Delete").clicked()
                && let Some(old) = self.selected.clone()
            {
                match self.store.delete(&old) {
                    Ok(()) => {
                        self.selected = None;
                        self.dirty = true;
                    }
                    Err(error) => self.notice = Some(error),
                }
            }
        });
        if let Some(notice) = &self.notice {
            ui.label(notice);
        }
        applied
    }

    fn commit_save(&mut self, preferences: ViewPreferences, overwrite: bool) {
        let name = self.name.trim().to_owned();
        if !overwrite && self.store.get(&name).is_some() {
            self.notice = Some("Named view already exists; use Overwrite".into());
            return;
        }
        match self.store.save(name.clone(), preferences) {
            Ok(()) => {
                self.selected = Some(name);
                self.dirty = true;
                self.notice = None;
            }
            Err(error) => self.notice = Some(error),
        }
    }
}

pub fn storage_key(scope: &NamedViewScope) -> Result<String, String> {
    let encoded =
        serde_json::to_string(scope).map_err(|_| "Unable to encode named-view scope".to_owned())?;
    Ok(format!("named-views-v1:{encoded}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MemoryStorage {
        values: std::collections::BTreeMap<String, String>,
        writes: usize,
    }
    impl Storage for MemoryStorage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.values.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.writes += 1;
            self.values.insert(key.into(), value);
        }
        fn remove_string(&mut self, key: &str) {
            self.values.remove(key);
        }
        fn flush(&mut self) {}
    }

    #[test]
    fn named_views_reopen_isolated_and_unchanged_storage_is_not_rewritten() {
        let scope = NamedViewScope::new("account-a", "products", "main");
        let mut ui = NamedViewsUi::new(scope.clone()).unwrap();
        let view =
            crate::table::TableView::new(&scope.preference_scope(), crate::data::Kind::Products);
        ui.name = "Daily".into();
        ui.commit_save(view.preferences(), false);
        let mut storage = MemoryStorage::default();
        ui.persist(&mut storage).unwrap();
        ui.persist(&mut storage).unwrap();
        assert_eq!(storage.writes, 1);
        let reopened = NamedViewsUi::load(&storage, scope.clone()).unwrap();
        assert!(reopened.store.get("Daily").is_some());
        let other = NamedViewScope::new("account-b", "products", "main");
        assert!(
            NamedViewsUi::load(&storage, other.clone())
                .unwrap()
                .store
                .list()
                .is_empty()
        );
        let original = storage.get_string(&storage_key(&scope).unwrap()).unwrap();
        storage
            .values
            .insert(storage_key(&other).unwrap(), original);
        assert!(NamedViewsUi::load(&storage, other).is_err());
        let key = storage_key(&scope).unwrap();
        storage.values.insert(key.clone(), "{".into());
        assert!(NamedViewsUi::load(&storage, scope).is_err());
        assert_eq!(storage.values[&key], "{");
        assert_eq!(storage.writes, 1);
    }
}
