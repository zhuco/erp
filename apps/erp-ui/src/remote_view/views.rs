use crate::named_views::NamedViewsUi;
use eframe::{Storage, egui};
use erp_grid_core::{NamedViewScope, RowId, ViewPreferences};

const PAGE: &str = "products-schema1";
const INSTANCE: &str = "main";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AccountIdentity {
    server: String,
    tenant: RowId,
    actor: RowId,
}

impl AccountIdentity {
    pub(super) fn new(server: &str, tenant: RowId, actor: RowId) -> Self {
        Self {
            server: server.to_owned(),
            tenant,
            actor,
        }
    }

    pub(super) fn scope(&self, page: &str, instance: &str) -> NamedViewScope {
        let tenant = self.tenant.to_uuid().to_string();
        let actor = self.actor.to_uuid().to_string();
        let account = format!(
            "s{}:{}t{}:{}a{}:{}",
            self.server.len(),
            self.server,
            tenant.len(),
            tenant,
            actor.len(),
            actor
        );
        NamedViewScope::new(account, page, instance)
    }
}

/// Account-bound host for a remote grid's saved views.
///
/// It owns at most one account store. Call `sync` after authentication state is
/// updated, `ui` while drawing its toolbar, and `persist` from the app
/// save hook (and after UI mutations when storage is available).
#[derive(Default)]
pub(super) struct Views {
    grid: Option<(&'static str, &'static str)>,
    identity: Option<AccountIdentity>,
    named: Option<NamedViewsUi>,
    open: bool,
    error: Option<String>,
    storage_available: bool,
}

impl Views {
    pub(super) fn for_grid(page: &'static str, instance: &'static str) -> Self {
        Self {
            grid: Some((page, instance)),
            ..Default::default()
        }
    }

    pub(super) fn sync(
        &mut self,
        storage: &mut dyn Storage,
        identity: Option<(&str, RowId, RowId)>,
    ) {
        self.storage_available = true;
        let next =
            identity.map(|(server, tenant, actor)| AccountIdentity::new(server, tenant, actor));
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
            self.error = Some(format!("Saved views could not be stored: {error}"));
            return;
        }

        self.identity = next;
        self.named = None;
        self.open = false;
        self.error = None;
        let Some(identity) = &self.identity else {
            return;
        };
        let (page, instance) = self.grid.unwrap_or((PAGE, INSTANCE));
        match NamedViewsUi::load(storage, identity.scope(page, instance)) {
            Ok(named) => self.named = Some(named),
            Err(error) => {
                // Keep the invalid value untouched. In particular, do not create
                // a fresh dirty store that could overwrite recoverable data.
                self.error = Some(format!("Saved views unavailable: {error}"));
            }
        }
    }

    pub(super) fn set_unavailable(&mut self) {
        self.storage_available = false;
        self.open = false;
        self.error = Some("Saved views require local storage".into());
    }

    pub(super) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        current: &ViewPreferences,
        enabled: bool,
    ) -> Option<ViewPreferences> {
        let (page, instance) = self.grid.unwrap_or((PAGE, INSTANCE));
        let expected_scope = self
            .identity
            .as_ref()
            .map(|identity| identity.scope(page, instance).preference_scope());
        let available = self.storage_available
            && self.named.is_some()
            && expected_scope.as_deref() == Some(current.scope.as_str());
        if ui
            .add_enabled(enabled && available, egui::Button::new("视图"))
            .on_hover_text(
                self.error
                    .as_deref()
                    .unwrap_or("Save or apply views for this account"),
            )
            .clicked()
        {
            self.open = true;
        }

        let mut applied = None;
        if self.open && available {
            egui::Window::new("保存视图")
                .id(ui.id().with((page, instance, "remote-views")))
                .open(&mut self.open)
                .show(ui.ctx(), |ui| {
                    ui.add_enabled_ui(enabled, |ui| {
                        if let Some(named) = &mut self.named {
                            applied = named.ui(ui, current);
                        }
                    });
                });
        }
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::YELLOW, "!")
                .on_hover_text(error);
        }
        applied
    }

    pub(super) fn scope(&self) -> Option<String> {
        if !self.storage_available {
            return None;
        }
        let (page, instance) = self.grid.unwrap_or((PAGE, INSTANCE));
        self.identity
            .as_ref()
            .map(|identity| identity.scope(page, instance).preference_scope())
    }

    pub(super) fn persist(&mut self, storage: &mut dyn Storage) -> Result<(), String> {
        if let Some(named) = &mut self.named {
            named.persist(storage)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::named_views::storage_key;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct MemoryStorage {
        values: BTreeMap<String, String>,
        writes: usize,
    }

    impl Storage for MemoryStorage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.values.get(key).cloned()
        }

        fn set_string(&mut self, key: &str, value: String) {
            self.writes += 1;
            self.values.insert(key.to_owned(), value);
        }

        fn remove_string(&mut self, key: &str) {
            self.values.remove(key);
        }

        fn flush(&mut self) {}
    }

    #[test]
    fn page_and_instance_stores_are_isolated_for_the_same_account() {
        let mut storage = MemoryStorage::default();
        let identity = AccountIdentity::new("https://erp.example", RowId(1), RowId(2));
        let order_key = storage_key(&identity.scope("orders-schema1", "main")).unwrap();
        storage.values.insert(order_key.clone(), "{".into());
        let mut products = Views::default();
        let mut orders = Views::for_grid("orders-schema1", "main");
        let mut detail = Views::for_grid("orders-schema1", "items");
        for view in [&mut products, &mut orders, &mut detail] {
            view.sync(
                &mut storage,
                Some((&identity.server, identity.tenant, identity.actor)),
            );
        }
        assert!(products.named.is_some());
        assert!(detail.named.is_some());
        assert!(orders.named.is_none());
        assert_ne!(products.scope(), detail.scope());
        orders.persist(&mut storage).unwrap();
        assert_eq!(storage.values[&order_key], "{");
    }

    #[test]
    fn account_scope_separates_actor_tenant_and_server_without_ambiguity() {
        let base =
            AccountIdentity::new("https://erp.example/a", RowId(1), RowId(2)).scope(PAGE, INSTANCE);
        let actor =
            AccountIdentity::new("https://erp.example/a", RowId(1), RowId(3)).scope(PAGE, INSTANCE);
        let tenant =
            AccountIdentity::new("https://erp.example/a", RowId(2), RowId(2)).scope(PAGE, INSTANCE);
        let server =
            AccountIdentity::new("https://erp.example", RowId(1), RowId(2)).scope(PAGE, INSTANCE);
        assert_ne!(storage_key(&base).unwrap(), storage_key(&actor).unwrap());
        assert_ne!(storage_key(&base).unwrap(), storage_key(&tenant).unwrap());
        assert_ne!(storage_key(&base).unwrap(), storage_key(&server).unwrap());
        assert_eq!(base.page, PAGE);
        assert_eq!(base.instance, INSTANCE);
    }

    #[test]
    fn identity_change_loads_once_and_corrupt_data_is_left_untouched() {
        let mut storage = MemoryStorage::default();
        let corrupt = AccountIdentity::new("https://erp.example", RowId(1), RowId(2));
        let key = storage_key(&corrupt.scope(PAGE, INSTANCE)).unwrap();
        storage.values.insert(key.clone(), "{".into());

        let mut views = Views::default();
        views.sync(
            &mut storage,
            Some((&corrupt.server, corrupt.tenant, corrupt.actor)),
        );
        views.sync(
            &mut storage,
            Some((&corrupt.server, corrupt.tenant, corrupt.actor)),
        );
        views.persist(&mut storage).unwrap();

        assert!(views.named.is_none());
        assert!(views.error.is_some());
        assert_eq!(storage.values[&key], "{");
        assert_eq!(storage.writes, 0);
    }

    #[test]
    fn unavailable_hides_scope_without_discarding_loaded_account() {
        let mut storage = MemoryStorage::default();
        let identity = AccountIdentity::new("https://erp.example", RowId(1), RowId(2));
        let mut views = Views::default();
        views.sync(
            &mut storage,
            Some((&identity.server, identity.tenant, identity.actor)),
        );
        assert!(views.named.is_some());
        assert!(views.scope().is_some());

        views.set_unavailable();

        assert!(views.named.is_some());
        assert_eq!(views.identity, Some(identity));
        assert!(views.scope().is_none());
        assert!(!views.open);
    }
}
