use super::*;
use crate::remote_options;
use erp_grid_core::{
    FilterDraft, OptionDirectory, OptionIdentity, OptionRequest, paged_query::PageQuery,
};
use std::collections::{BTreeMap, BTreeSet};

type Binding = (DataScope, PageQuery, Option<String>, u64);
#[derive(Default)]
struct Entry {
    binding: Option<Binding>,
    directory: OptionDirectory,
}
struct Job {
    column: ColumnId,
    request: OptionRequest,
    wire: remote_options::OptionHttpQuery,
    task: HttpTask,
}

pub(super) struct ProductOptions {
    entries: BTreeMap<ColumnId, Entry>,
    job: Option<Job>,
    epoch: u64,
    pub open: bool,
    column: ColumnId,
    search: String,
    offset: u64,
    deadline: Option<f64>,
    composing: bool,
}

impl Default for ProductOptions {
    fn default() -> Self {
        Self {
            entries: (2..=4).map(|id| (ColumnId(id), Entry::default())).collect(),
            job: None,
            epoch: 0,
            open: false,
            column: ColumnId(2),
            search: String::new(),
            offset: 0,
            deadline: None,
            composing: false,
        }
    }
}
impl ProductOptions {
    pub(super) fn open_column(&mut self, column: ColumnId) {
        self.column = column;
        self.open = true;
    }
    pub fn clear(&mut self) {
        self.job = None;
        for entry in self.entries.values_mut() {
            entry.binding = None;
            entry.directory.cancel();
        }
    }

    /// Returns true when authentication failed; caller must expire its session.
    pub fn tick(&mut self, ctx: &egui::Context, session: &Session, query: &RemoteQuery) -> bool {
        ctx.input(|input| {
            for event in &input.events {
                if let egui::Event::Ime(event) = event {
                    match event {
                        egui::ImeEvent::Preedit { text, .. } => self.composing = !text.is_empty(),
                        egui::ImeEvent::Commit(_) => self.composing = false,
                        _ => {}
                    }
                }
            }
        });
        if self.deadline.is_some_and(|at| ctx.input(|i| i.time) < at) || self.composing {
            ctx.request_repaint_after(Duration::from_millis(50));
            return false;
        }
        self.deadline = None;
        for (&column, entry) in &mut self.entries {
            let mut filters = query.filters.clone();
            filters.remove(&column);
            let page = PageQuery {
                expression: query.expression.as_deref().cloned(),
                schema: query.scope.schema,
                offset: 0,
                limit: 100,
                filters,
                sorts: vec![],
                search: query.search.clone(),
            };
            let (search, offset) = if self.open && self.column == column {
                (
                    (!self.search.trim().is_empty()).then(|| self.search.trim().to_owned()),
                    self.offset,
                )
            } else {
                (None, 0)
            };
            let binding = (query.scope.clone(), page, search, offset);
            if entry.binding.as_ref() != Some(&binding) {
                if self.job.as_ref().is_some_and(|job| job.column == column) {
                    self.job = None;
                }
                self.epoch = self.epoch.wrapping_add(1);
                entry.directory.bind(OptionIdentity {
                    tenant: session.tenant,
                    query_scope: format!("{}:{}", query.scope.source, query.scope.authorization),
                    filter_scope: self.epoch.to_string(),
                    column,
                    search: binding.2.clone(),
                    offset,
                });
                entry.binding = Some(binding);
            }
        }
        if let Some(result) = self.job.as_mut().and_then(|job| job.task.poll()) {
            let Some(job) = self.job.take() else {
                return false;
            };
            let denied = matches!(&result, Err(HttpError::Status(401 | 403)));
            let response = result
                .map_err(|error| error.to_string())
                .and_then(|bytes| remote_options::decode_options(&job.wire, &bytes));
            if let Some(entry) = self.entries.get_mut(&job.column) {
                entry.directory.accept(&job.request, response);
            }
            if denied {
                self.clear();
                return true;
            }
        }
        if self.job.is_none() {
            let mut order = vec![self.column, ColumnId(2), ColumnId(3), ColumnId(4)];
            order.dedup();
            if let Some(column) = order.into_iter().find(|id| {
                self.entries.get(id).is_some_and(|entry| {
                    entry.directory.total.is_none()
                        && !entry.directory.loading
                        && entry.directory.error.is_none()
                })
            }) && let Some(entry) = self.entries.get_mut(&column)
                && let Some(binding) = &entry.binding
            {
                match entry.directory.start(100) {
                    Ok(request) => {
                        let prepared = remote_options::encode_options(
                            request.identity.clone(),
                            binding.1.clone(),
                            request.limit,
                        )
                        .and_then(|wire| {
                            let task = session
                                .client
                                .start(
                                    HttpRequest {
                                        path: wire.path.clone(),
                                        query: vec![],
                                        json: Some(wire.body.clone()),
                                        bearer: Some(session.token.clone()),
                                    },
                                    ctx,
                                )
                                .map_err(|error| error.to_string())?;
                            Ok((wire, task))
                        });
                        match prepared {
                            Ok((wire, task)) => {
                                self.job = Some(Job {
                                    column,
                                    request,
                                    wire,
                                    task,
                                })
                            }
                            Err(error) => {
                                entry.directory.accept(&request, Err(error));
                            }
                        }
                    }
                    Err(error) => entry.directory.error = Some(error),
                }
            }
        }
        if self.job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(25));
        }
        false
    }

    pub fn update_columns(
        &self,
        columns: &mut [Column],
        drafts: &mut BTreeMap<ColumnId, FilterDraft>,
    ) {
        for column in columns.iter_mut().filter(|c| (2..=4).contains(&c.id.0)) {
            let Some(entry) = self.entries.get(&column.id) else {
                continue;
            };
            let directory = &entry.directory;
            if directory.loading || directory.total.is_none() || directory.error.is_some() {
                continue;
            }
            if matches!(drafts.get(&column.id), Some(FilterDraft::Text(text)) if !text.is_empty()) {
                continue;
            }
            let selected = match drafts.get(&column.id) {
                Some(FilterDraft::Enum(values)) => values.clone(),
                _ => BTreeSet::new(),
            };
            if directory.complete_dropdown() || !selected.is_empty() {
                let mut values = selected;
                if directory.complete_dropdown() {
                    values.extend(directory.values.iter().cloned());
                }
                column.filter = FilterKind::Enum(values.into_iter().collect());
                if !matches!(drafts.get(&column.id), Some(FilterDraft::Enum(_))) {
                    drafts.insert(column.id, FilterDraft::Enum(BTreeSet::new()));
                }
            } else if !self.open || self.column != column.id {
                column.filter = FilterKind::Text;
                drafts.insert(column.id, FilterDraft::Text(String::new()));
            }
        }
    }

    /// Changes return immediately; the parent applies its normal query debounce.
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        drafts: &BTreeMap<ColumnId, FilterDraft>,
        enabled: bool,
    ) -> Option<(ColumnId, BTreeSet<String>)> {
        if !self.open {
            return None;
        }
        let mut open = self.open;
        let mut changed = None;
        let mut invalidate = false;
        egui::Window::new("Filter values")
            .id(egui::Id::new("product-filter-values"))
            .open(&mut open)
            .show(ui.ctx(), |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    let old_column = self.column;
                    egui::ComboBox::from_id_salt("column")
                        .selected_text(match self.column.0 {
                            2 => "Brand",
                            3 => "Category",
                            _ => "Status",
                        })
                        .show_ui(ui, |ui| {
                            for (id, title) in [(2, "Brand"), (3, "Category"), (4, "Status")] {
                                ui.selectable_value(&mut self.column, ColumnId(id), title);
                            }
                        });
                    if self.column != old_column {
                        self.search.clear();
                        self.offset = 0;
                        invalidate = true;
                    }
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.search)
                                .hint_text("Search values")
                                .char_limit(256),
                        )
                        .changed()
                    {
                        self.offset = 0;
                        self.deadline = Some(ui.input(|i| i.time) + 0.25);
                        invalidate = true;
                    }
                    if let Some(entry) = self.entries.get(&self.column) {
                        let directory = &entry.directory;
                        if directory.loading || self.deadline.is_some() {
                            ui.spinner();
                        }
                        if let Some(error) = &directory.error {
                            ui.label(error);
                            if ui.button("Retry").clicked() {
                                invalidate = true;
                            }
                        }
                        let mut selected = match drafts.get(&self.column) {
                            Some(FilterDraft::Enum(values)) => values.clone(),
                            _ => BTreeSet::new(),
                        };
                        if ui.button("Clear values").clicked() {
                            selected.clear();
                            changed = Some((self.column, selected.clone()));
                        }
                        ui.add_enabled_ui(
                            !directory.loading
                                && directory.error.is_none()
                                && !invalidate
                                && directory.total.is_some(),
                            |ui| {
                                egui::ScrollArea::vertical()
                                    .max_height(240.0)
                                    .show(ui, |ui| {
                                        for value in &directory.values {
                                            let mut checked = selected.contains(value);
                                            if ui
                                                .add_enabled(
                                                    checked || selected.len() < 100,
                                                    egui::Checkbox::new(
                                                        &mut checked,
                                                        if value.is_empty() {
                                                            "(Empty)"
                                                        } else {
                                                            value
                                                        },
                                                    ),
                                                )
                                                .changed()
                                            {
                                                if checked {
                                                    selected.insert(value.clone());
                                                } else {
                                                    selected.remove(value);
                                                }
                                                changed = Some((self.column, selected.clone()));
                                            }
                                        }
                                    });
                                ui.horizontal(|ui| {
                                    if ui
                                        .add_enabled(self.offset > 0, egui::Button::new("First"))
                                        .clicked()
                                    {
                                        self.offset = 0;
                                        invalidate = true;
                                    }
                                    if ui
                                        .add_enabled(self.offset > 0, egui::Button::new("Previous"))
                                        .clicked()
                                    {
                                        self.offset = self.offset.saturating_sub(100);
                                        invalidate = true;
                                    }
                                    let next =
                                        self.offset.saturating_add(directory.values.len() as u64);
                                    if ui
                                        .add_enabled(
                                            !directory.values.is_empty()
                                                && directory
                                                    .total
                                                    .is_some_and(|total| next < total),
                                            egui::Button::new("Next"),
                                        )
                                        .clicked()
                                    {
                                        self.offset = next;
                                        invalidate = true;
                                    }
                                    if let Some(total) = directory.total {
                                        ui.label(format!("{total} values"));
                                    }
                                });
                            },
                        );
                    }
                });
            });
        self.open = open;
        if invalidate {
            self.clear();
            ui.ctx().request_repaint();
        }
        changed
    }
}

#[cfg(test)]
#[path = "options_tests.rs"]
mod tests;
