use eframe::egui;
use erp_grid_core::RowId;
pub mod action_store;
pub mod components;
mod data;
mod draft_store;
mod edit_drafts;
pub mod erp_app;
mod erp_theme;
mod export;
pub mod first_load;
mod fonts;
pub mod http;
mod leave_guard;
pub mod module_demo;
mod named_views;
pub mod page_lab;
pub mod performance;
mod query;
pub mod remote_actions;
mod remote_copy_batches;
pub mod remote_options;
mod remote_paste;
pub mod remote_products;
pub mod remote_selection;
mod remote_view;
mod report_view;
mod selection_summary;
mod shared_edits;
mod table;
mod tree_view;
pub mod update_lab;
pub mod workbench;
mod workspace_shell;
use data::Kind;
use std::collections::BTreeMap;
use table::TableView;

pub struct GridLab {
    remote: remote_view::RemoteProductView,
    page: leave_guard::Destination,
    tree: tree_view::TreeView,
    report: report_view::ReportView,
    main: TableView,
    goods: TableView,
    logs: TableView,
    tab: usize,
    details_height: f32,
    bound_parent: Option<RowId>,
    pending_parent: Option<Option<RowId>>,
    audit: BTreeMap<RowId, Vec<String>>,
    last_save: f64,
    leave_guard: leave_guard::LeaveGuard,
    allow_close: bool,
    workbook_delivery: Option<export::Delivery>,
    export_sample_image: bool,
}
impl GridLab {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        fonts::install(&cc.egui_ctx);
        let mut app = Self {
            remote: Default::default(),
            page: leave_guard::Destination::Lab,
            tree: Default::default(),
            report: Default::default(),
            main: TableView::new(&lab_view_scope("main").preference_scope(), Kind::Products),
            goods: TableView::new(&lab_view_scope("goods").preference_scope(), Kind::Goods),
            logs: TableView::new(&lab_view_scope("logs").preference_scope(), Kind::Logs),
            tab: 0,
            details_height: 270.0,
            bound_parent: None,
            pending_parent: None,
            audit: BTreeMap::new(),
            last_save: 0.0,
            leave_guard: Default::default(),
            allow_close: false,
            workbook_delivery: None,
            export_sample_image: false,
        };
        app.goods.data.edits = app.main.detail_edits.clone();
        for (instance, view) in [
            ("main", &mut app.main),
            ("goods", &mut app.goods),
            ("logs", &mut app.logs),
        ] {
            let scope = lab_view_scope(instance);
            let loaded = if let Some(storage) = cc.storage {
                named_views::NamedViewsUi::load(storage, scope)
            } else {
                named_views::NamedViewsUi::new(scope)
            };
            match loaded {
                Ok(named) => view.named_views = Some(named),
                Err(error) => {
                    view.notice = Some(format!("Saved views unavailable: {error}"));
                }
            }
        }
        if let Some(storage) = cc.storage {
            for view in [&mut app.main, &mut app.goods, &mut app.logs] {
                if let Some(text) = storage.get_string(&view.scope)
                    && let Err(error) = view.restore(&text)
                {
                    view.notice = Some(format!("Settings reset: {error}"));
                }
            }
        }
        app
    }
    fn bind(&mut self, parent: Option<RowId>) {
        self.bound_parent = parent;
        self.goods.bind_parent(parent, Vec::new());
        self.logs.bind_parent(
            parent,
            parent
                .and_then(|id| self.audit.get(&id).cloned())
                .unwrap_or_default(),
        );
    }
    fn record_events(&mut self) {
        if let Some((targets, message)) = self.main.event.take() {
            for row in targets {
                let events = self.audit.entry(row).or_default();
                events.push(message.clone());
                if events.len() > 100 {
                    events.remove(0);
                }
            }
        }
        if let Some((_, message)) = self.goods.event.take()
            && let Some(parent) = self.bound_parent
        {
            let events = self.audit.entry(parent).or_default();
            events.push(format!("Goods: {message}"));
            if events.len() > 100 {
                events.remove(0);
            }
        }
        for (parent, message) in self.main.take_detail_events() {
            let events = self.audit.entry(parent).or_default();
            events.push(format!("Goods: {message}"));
            if events.len() > 100 {
                events.remove(0);
            }
        }
        while self.audit.len() > 1000 {
            self.audit.pop_first();
        }
        self.logs.bind_parent(
            self.bound_parent,
            self.bound_parent
                .and_then(|id| self.audit.get(&id).cloned())
                .unwrap_or_default(),
        );
    }

    fn workbook_export(
        &self,
    ) -> Result<(Vec<export::WorkbookSheet>, Vec<export::SheetOptions>), String> {
        let mut sheets = vec![self.main.workbook_page("Products")?];
        if self.bound_parent.is_some() {
            sheets.push(if self.tab == 0 {
                self.goods.workbook_page("Goods")?
            } else {
                self.logs.workbook_page("Logs")?
            });
        }
        workbook_with_sample_options(sheets, self.export_sample_image)
    }

    fn export_workbook(&mut self) -> Result<(), String> {
        let (sheets, options) = self.workbook_export()?;
        let bytes = export::encode_workbook_with_options(&sheets, &options)?;
        self.workbook_delivery = Some(export::Delivery::start(bytes, export::Format::Xlsx));
        Ok(())
    }

    fn apply_destination(&mut self, destination: leave_guard::Destination, ctx: &egui::Context) {
        if self.page == leave_guard::Destination::Products && destination != self.page {
            self.remote.suspend_page_queries();
        }
        if self.page == leave_guard::Destination::Orders && destination != self.page {
            self.remote.close_orders_page();
        }
        match destination {
            leave_guard::Destination::Close => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            page => self.page = page,
        }
    }

    fn hold_for_leave(&mut self) {
        self.main.hold_for_leave();
        self.goods.hold_for_leave();
        self.remote.hold_for_leave();
    }

    fn finish_leave_dialog(&mut self, ctx: &egui::Context) {
        if !self.leave_guard.pending() {
            return;
        }
        let dirty = self.main.dirty() || self.goods.dirty() || self.remote.dirty();
        if self.leave_guard.saving() && !dirty {
            if let Some(destination) = self.leave_guard.resolve(leave_guard::Choice::Save) {
                self.leave_guard.sync_before_unload(false);
                self.apply_destination(destination, ctx);
            }
            return;
        }
        if self.leave_guard.saving() && self.remote.leave_blocked().is_none() {
            self.leave_guard.save_failed(
                "Save was not confirmed. Resolve the validation or conflict in the editor, then retry."
                    .into(),
            );
        }
        let blocked = if self.leave_guard.saving() {
            None
        } else {
            self.remote.leave_blocked()
        };
        let Some(choice) = self.leave_guard.dialog(ctx, blocked) else {
            return;
        };
        match choice {
            leave_guard::Choice::KeepEditing => {
                self.leave_guard.resolve(choice);
            }
            leave_guard::Choice::Discard => {
                if self.remote.discard_for_leave().is_err() {
                    return;
                }
                self.main.discard_draft();
                self.goods.discard_draft();
                if let Some(destination) = self.leave_guard.resolve(choice) {
                    self.leave_guard.sync_before_unload(false);
                    self.apply_destination(destination, ctx);
                }
            }
            leave_guard::Choice::Save => {
                let local = self
                    .main
                    .save_for_leave()
                    .and_then(|ready| self.goods.save_for_leave().map(|detail| ready && detail));
                let remote = self.remote.request_save_for_leave(ctx);
                match (local, remote) {
                    (Ok(true), Ok(true)) => {
                        if let Some(destination) = self.leave_guard.resolve(choice) {
                            self.leave_guard.sync_before_unload(false);
                            self.apply_destination(destination, ctx);
                        }
                    }
                    (Err(error), _) => self.leave_guard.save_failed(error),
                    (_, Err(error)) => self.leave_guard.save_failed(error),
                    _ => self.leave_guard.wait_for_save(),
                }
            }
        }
    }
}
impl eframe::App for GridLab {
    fn ui(&mut self, root: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.remote.tick_orders(root.ctx());
        if let Some(delivery) = &self.workbook_delivery {
            if let Some(result) = delivery.poll() {
                self.main.notice = Some(result.unwrap_or_else(|error| error));
                self.workbook_delivery = None;
            } else {
                root.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
        let dirty = self.main.dirty() || self.goods.dirty() || self.remote.dirty();
        self.leave_guard.sync_before_unload(dirty);
        if root.input(|i| i.viewport().close_requested()) && !self.allow_close {
            root.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if dirty && !self.leave_guard.pending() {
                self.hold_for_leave();
            }
            if let Some(destination) = self
                .leave_guard
                .request(leave_guard::Destination::Close, dirty)
            {
                self.apply_destination(destination, root.ctx());
            }
        }
        let mut navigation = None;
        egui::Panel::top("title").show(root, |ui| {
            ui.horizontal(|ui| {
                let lab = ui.selectable_label(
                    self.page == leave_guard::Destination::Lab,
                    "Lab",
                );
                if lab.is_pointer_button_down_on() && dirty && !self.leave_guard.pending() {
                    self.hold_for_leave();
                }
                if lab.clicked() {
                    navigation = Some(leave_guard::Destination::Lab);
                }
                let products = ui.selectable_label(self.page == leave_guard::Destination::Products, "Products API");
                if products.is_pointer_button_down_on() && dirty && !self.leave_guard.pending() {
                    self.hold_for_leave();
                }
                if products.clicked() {
                    navigation = Some(leave_guard::Destination::Products);
                }
                let orders = ui.selectable_label(self.page == leave_guard::Destination::Orders, "Orders API");
                if orders.is_pointer_button_down_on() && dirty && !self.leave_guard.pending() {
                    self.hold_for_leave();
                }
                if orders.clicked() {
                    navigation = Some(leave_guard::Destination::Orders);
                }
                let reports = ui.selectable_label(self.page == leave_guard::Destination::Reports, "Reports");
                if reports.is_pointer_button_down_on() && dirty && !self.leave_guard.pending() {
                    self.hold_for_leave();
                }
                if reports.clicked() {
                    navigation = Some(leave_guard::Destination::Reports);
                }
                let tree = ui.selectable_label(self.page == leave_guard::Destination::WarehouseTree, "Warehouse tree");
                if tree.is_pointer_button_down_on() && dirty && !self.leave_guard.pending() {
                    self.hold_for_leave();
                }
                if tree.clicked() {
                    navigation = Some(leave_guard::Destination::WarehouseTree);
                }
                ui.heading("ERP Grid").on_hover_text(if matches!(self.page, leave_guard::Destination::Products | leave_guard::Destination::Orders) {
                    "Product API connection with server paging and versioned editing."
                } else {
                    "Local synthetic data. Edits affect this session only; view settings are saved."
                });
                if self.page == leave_guard::Destination::Lab {
                    for count in [10, 10_000, 100_000, 1_000_000] {
                        if ui
                            .add_enabled(
                                !self.main.dirty() && !self.goods.dirty(),
                                egui::Button::new(format!("{count} rows"))
                                    .selected(self.main.data.rows == count),
                            )
                            .clicked()
                        {
                            self.main.set_rows(count);
                        }
                    }
                    let detail = if self.tab == 0 { &self.goods } else { &self.logs };
                    let ready = !self.main.busy() && !self.main.dirty()
                        && !detail.busy() && !detail.dirty()
                        && self.workbook_delivery.is_none();
                    ui.checkbox(&mut self.export_sample_image, "Images")
                        .on_hover_text("local sample image, not product photos");
                    if ui.add_enabled(ready, egui::Button::new("Export workbook"))
                        .on_hover_text("Export the current product page and the selected product's visible bottom detail page to separate Excel sheets")
                        .clicked()
                        && let Err(error) = self.export_workbook()
                    {
                        self.main.notice = Some(error);
                    }
                }
            });
        });
        // Keyboard activation or a press/release delivered in one frame may
        // have no held-pointer state; freeze before drawing the editor too.
        if navigation.is_some() && dirty && !self.leave_guard.pending() {
            self.hold_for_leave();
        }
        if let Some(destination) = navigation
            && let Some(destination) = self.leave_guard.request(destination, dirty)
        {
            self.apply_destination(destination, root.ctx());
        }
        if self.page == leave_guard::Destination::WarehouseTree {
            egui::CentralPanel::default().show(root, |ui| self.tree.ui(ui));
        } else if self.page == leave_guard::Destination::Reports {
            egui::CentralPanel::default().show(root, |ui| self.report.ui(ui));
        } else if matches!(
            self.page,
            leave_guard::Destination::Products | leave_guard::Destination::Orders
        ) {
            if let Some(storage) = frame.storage_mut() {
                self.remote.sync_views(storage);
            } else {
                self.remote.views_storage_unavailable();
            }
            egui::CentralPanel::default().show(root, |ui| {
                if self.page == leave_guard::Destination::Orders {
                    self.remote.orders_ui(ui);
                } else {
                    self.remote.ui(ui);
                }
            });
            if let Some(storage) = frame.storage_mut() {
                self.remote.sync_views(storage);
                self.remote.persist_views(storage);
            }
        } else {
            let bounds = root.available_rect_before_wrap();
            let max_height = (bounds.height() - 120.0).max(160.0);
            let height = self.details_height.clamp(160.0, max_height);
            let panel =
                egui::Panel::bottom("details")
                    .resizable(false)
                    .exact_size(height)
                    .show(root, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(self.bound_parent.map_or_else(
                                || "Details".into(),
                                |id| format!("Details #{}", id.0),
                            ));
                            ui.selectable_value(&mut self.tab, 0, "Goods");
                            ui.selectable_value(&mut self.tab, 1, "Logs");
                        });
                        if self.tab == 0 {
                            self.goods.ui(ui);
                        } else {
                            self.logs.ui(ui);
                        }
                    });
            let edge = panel.response.rect.top();
            let handle = egui::Rect::from_min_max(
                egui::pos2(bounds.left(), edge - 4.0),
                egui::pos2(bounds.right(), edge + 4.0),
            );
            let response = root
                .interact(handle, egui::Id::new("details-height"), egui::Sense::drag())
                .on_hover_cursor(egui::CursorIcon::ResizeVertical);
            if (response.dragged() || response.drag_stopped())
                && let Some(pointer) = response.interact_pointer_pos()
            {
                self.details_height = (bounds.bottom() - pointer.y).clamp(160.0, max_height);
                root.ctx().request_repaint();
            }
            egui::CentralPanel::default().show(root, |ui| self.main.ui(ui));
            let requested = self.main.state.row_selection.active;
            if requested != self.bound_parent {
                if self.goods.dirty() {
                    self.pending_parent = Some(requested);
                    self.main.state.row_selection.active = self.bound_parent;
                } else {
                    self.bind(requested);
                }
            }
            if let Some(parent) = self.pending_parent {
                let mut discard = false;
                let mut keep = false;
                egui::Window::new("Unsaved detail changes")
                    .collapsible(false)
                    .show(root.ctx(), |ui| {
                        if ui.button("Keep editing").clicked() {
                            keep = true;
                        }
                        if ui.button("Discard draft and switch").clicked() {
                            discard = true;
                        }
                    });
                if keep {
                    self.pending_parent = None;
                }
                if discard {
                    self.goods.discard_draft();
                    self.main.state.row_selection.active = parent;
                    self.bind(parent);
                    self.pending_parent = None;
                }
            }
            self.record_events();
        }
        self.finish_leave_dialog(root.ctx());
        let now = root.input(|i| i.time);
        if now - self.last_save >= 2.0 {
            if let Some(storage) = frame.storage_mut() {
                self.save(storage);
                storage.flush();
            }
            self.last_save = now;
        }
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.remote.persist_views(storage);
        for view in [&mut self.main, &mut self.goods, &mut self.logs] {
            if let Some(named) = &mut view.named_views
                && let Err(error) = named.persist(storage)
            {
                view.notice = Some(format!("Saved views could not be stored: {error}"));
            }
            match serde_json::to_string(&view.preferences()) {
                Ok(text) => storage.set_string(&view.scope, text),
                Err(error) => view.notice = Some(format!("Settings save failed: {error}")),
            }
        }
    }
    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(2)
    }
}

fn workbook_with_sample_options(
    sheets: Vec<export::WorkbookSheet>,
    include_sample: bool,
) -> Result<(Vec<export::WorkbookSheet>, Vec<export::SheetOptions>), String> {
    if sheets.is_empty() {
        return Err("Workbook must contain at least one sheet".into());
    }
    let mut options = vec![export::SheetOptions::default(); sheets.len()];
    if include_sample
        && sheets[0].name == "Products"
        && !sheets[0].columns.is_empty()
        && !sheets[0].rows.is_empty()
    {
        options[0].images.push(export::SheetImage {
            row: 0,
            column: 0,
            png: include_bytes!("../assets/export-sample.png").to_vec(),
            width_px: 64,
            height_px: 64,
        });
    }
    Ok((sheets, options))
}

fn lab_view_scope(instance: &str) -> erp_grid_core::NamedViewScope {
    erp_grid_core::NamedViewScope::new("local-lab", "products", instance)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub async fn start_web() -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::{JsCast, JsValue};
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("document unavailable"))?;
    let canvas = document
        .get_element_by_id("erp-grid")
        .ok_or_else(|| JsValue::from_str("erp-grid canvas missing"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    let performance = document
        .body()
        .is_some_and(|body| body.has_attribute("data-grid-perf"));
    let updates = document
        .body()
        .is_some_and(|body| body.has_attribute("data-grid-updates"));
    let first_load = document
        .body()
        .is_some_and(|body| body.has_attribute("data-grid-first-load"));
    let modules = document
        .body()
        .is_some_and(|body| body.has_attribute("data-ui-modules"));
    let erp_app = document
        .body()
        .is_some_and(|body| body.has_attribute("data-erp-app"));
    let page_lab = document
        .body()
        .is_some_and(|body| body.has_attribute("data-grid-pages"));
    eframe::WebRunner::new()
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(move |cc| {
                if modules {
                    Ok(Box::new(module_demo::ModuleDemo::new(cc)))
                } else if erp_app {
                    Ok(Box::new(erp_app::ErpApp::new(cc)))
                } else if page_lab {
                    Ok(Box::new(
                        page_lab::PageLab::new(cc).map_err(std::io::Error::other)?,
                    ))
                } else if first_load {
                    Ok(Box::new(first_load::FirstLoadLab::new(cc)))
                } else if updates {
                    Ok(Box::new(
                        update_lab::UpdateLab::new(cc).map_err(std::io::Error::other)?,
                    ))
                } else if performance {
                    Ok(Box::new(performance::PerformanceLab::new(cc, false)))
                } else {
                    Ok(Box::new(GridLab::new(cc)))
                }
            }),
        )
        .await
}

#[cfg(test)]
mod workbook_export_tests {
    use super::*;

    #[test]
    fn workbook_options_reject_empty_and_respect_sample_choice() {
        assert!(workbook_with_sample_options(Vec::new(), true).is_err());
        let product = export::WorkbookSheet {
            name: "Products".into(),
            columns: vec![erp_grid_core::Column::new(
                erp_grid_core::ColumnId(7),
                "SKU",
                100.0,
            )],
            rows: vec![vec!["SKU-1".into()]],
        };
        let (_, without) = workbook_with_sample_options(vec![product.clone()], false).unwrap();
        assert!(without[0].images.is_empty());
        let (_, with) = workbook_with_sample_options(vec![product], true).unwrap();
        assert_eq!(with[0].images.len(), 1);
        assert_eq!((with[0].images[0].row, with[0].images[0].column), (0, 0));
    }
}
