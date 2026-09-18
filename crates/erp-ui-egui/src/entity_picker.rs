use egui::Id;
use erp_grid_core::{ColumnId, RowId};

/// A request belongs to one grid/editor/search revision. Hosts supply authorized results.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntitySearch {
    pub instance: Id,
    pub ticket: u64,
    pub row: RowId,
    pub column: ColumnId,
    pub revision: u64,
    pub search: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityOption {
    pub id: RowId,
    pub label: String,
}

#[derive(Default)]
pub(crate) struct Picker {
    pub query: String,
    pub options: Vec<EntityOption>,
    pub error: Option<String>,
    pub active: Option<EntitySearch>,
    pub outgoing: Option<EntitySearch>,
    pub selected: bool,
    pub initialized: bool,
    pub cursor: usize,
    pub loading: bool,
    revision: u64,
    deadline: f64,
    dirty: bool,
}

impl Picker {
    pub fn invalidate(&mut self, now: f64) {
        self.options.clear();
        self.loading = false;
        self.active = None;
        self.outgoing = None;
        self.selected = false;
        self.error = None;
        self.cursor = 0;
        self.dirty = true;
        self.deadline = now + 0.25;
    }

    pub fn poll(&mut self, mut request: EntitySearch, now: f64, composing: bool) {
        if !self.initialized {
            self.initialized = true;
            self.dirty = true;
        }
        if !self.dirty || composing || now < self.deadline {
            return;
        }
        self.revision = self.revision.wrapping_add(1);
        request.revision = self.revision;
        request.search = self.query.clone();
        self.active = Some(request.clone());
        self.loading = true;
        self.outgoing = Some(request);
        self.dirty = false;
    }

    pub fn resolve(
        &mut self,
        request: &EntitySearch,
        result: Result<Vec<EntityOption>, String>,
    ) -> bool {
        if self.active.as_ref() != Some(request) || !self.loading {
            return false;
        }
        self.loading = false;
        self.outgoing = None;
        let result = result.and_then(|options| {
            let mut ids = std::collections::BTreeSet::new();
            if options.len() > 100
                || options.iter().any(|o| {
                    o.id.0 == 0
                        || o.label.is_empty()
                        || o.label.len() > 4096
                        || o.label.contains('\0')
                        || !ids.insert(o.id)
                })
            {
                return Err("Invalid object options (limit 100)".into());
            }
            Ok(options)
        });
        match result {
            Ok(options) => {
                self.options = options;
                self.error = None;
            }
            Err(error) => {
                self.options.clear();
                self.error = Some(error.chars().take(512).collect());
            }
        }
        true
    }

    pub fn debounce_pending(&self) -> bool {
        self.dirty
    }

    pub fn choose(&mut self, index: usize) -> Option<String> {
        let option = self.options.get(index)?;
        let value = option.id.to_uuid();
        self.query.clone_from(&option.label);
        self.selected = true;
        self.loading = false;
        self.dirty = false;
        self.options.clear();
        self.active = None;
        self.outgoing = None;
        Some(value)
    }
}
