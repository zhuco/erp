//! Formal workspace host; GridLab remains a separate demonstration entry.
use crate::{
    leave_guard::{Choice, Destination, LeaveGuard},
    remote_view::RemoteProductView,
    workspace_shell::{Page, Shell, ShellEvent},
};
use eframe::egui;

pub struct ErpApp {
    shell: Shell,
    remote: RemoteProductView,
    reports: crate::report_view::ReportView,
    tree: crate::tree_view::TreeView,
    lab: crate::GridLab,
    leave: LeaveGuard,
    allow_close: bool,
    discard_waiting: bool,
    tab_saving: bool,
    tab_error: Option<String>,
    pending_refresh: Option<Page>,
}
impl ErpApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            shell: Shell::default(),
            remote: Default::default(),
            reports: Default::default(),
            tree: Default::default(),
            lab: crate::GridLab::new(cc),
            leave: Default::default(),
            allow_close: false,
            discard_waiting: false,
            tab_saving: false,
            tab_error: None,
            pending_refresh: None,
        };
        app.remote.set_formal_host(true);
        crate::erp_theme::install(&cc.egui_ctx);
        app
    }

    fn finish_page_intent(&mut self, page: Page) {
        if self.pending_refresh == Some(page) {
            self.pending_refresh = None;
            self.refresh_page(page);
        } else {
            self.shell.commit_close(page);
        }
        self.tab_saving = false;
        self.tab_error = None;
    }

    fn finish_discard(&mut self, ctx: &egui::Context) -> Result<bool, String> {
        if !self.remote.material_discard(ctx)? {
            return Ok(false);
        }
        if !self.remote.trade_discard(ctx)? {
            return Ok(false);
        }
        self.remote.discard_for_leave().map_err(str::to_owned)?;
        self.lab.remote.discard_for_leave().map_err(str::to_owned)?;
        self.lab.main.discard_draft();
        self.lab.goods.discard_draft();
        self.discard_waiting = false;
        self.leave.resolve(Choice::Discard);
        self.allow_close = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        Ok(true)
    }

    fn refresh_page(&mut self, page: Page) {
        match page {
            Page::Products | Page::Orders => self.remote.refresh_page(page == Page::Orders),
            Page::TradeSales => self.remote.trade_refresh(),
            Page::Materials => self.remote.material_refresh(),
            Page::Reports => self.reports = Default::default(),
            Page::WarehouseTree => self.tree = Default::default(),
            // The lab owns its own editing lifecycle and refresh controls.
            Page::Lab => {}
        }
    }
}
impl eframe::App for ErpApp {
    fn ui(&mut self, root: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.remote.background_tick(root.ctx());
        self.lab.remote.background_tick(root.ctx());
        if self.discard_waiting {
            match self.finish_discard(root.ctx()) {
                Ok(_) => {}
                Err(error) => {
                    self.discard_waiting = false;
                    self.leave.save_failed(error);
                }
            }
        }

        if let Some(storage) = frame.storage_mut() {
            let scope = self.remote.page_scope();
            self.shell.sync(storage, scope.as_deref());
        }
        let dirty = self.remote.dirty()
            || self.lab.main.dirty()
            || self.lab.goods.dirty()
            || self.lab.remote.dirty();
        self.leave.sync_before_unload(dirty);
        if root.input(|i| i.viewport().close_requested()) && !self.allow_close {
            root.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if self
                .leave
                .request_close(Destination::Close, dirty, false, false)
                .is_some()
            {
                self.allow_close = true;
                root.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                self.remote.hold_for_leave();
                self.lab.hold_for_leave();
            }
        }
        let products_open = self.shell.is_open(Page::Products);
        let orders_open = self.shell.is_open(Page::Orders);
        let trade_open = self.shell.is_open(Page::TradeSales);
        let material_open = self.shell.is_open(Page::Materials);
        let remote = &self.remote;
        let lab_dirty = self.lab.main.dirty() || self.lab.goods.dirty() || self.lab.remote.dirty();
        let event = self.shell.show(root, |page| match page {
            Page::Products => remote.page_dirty(false),
            Page::Orders => remote.page_dirty(true),
            Page::TradeSales => remote.trade_dirty(),
            Page::Materials => remote.material_dirty(),
            Page::Lab => lab_dirty,
            _ => false,
        });
        if matches!(event, Some(ShellEvent::Activate(_))) {
            self.remote.hold_for_leave();
            self.lab.hold_for_leave();
        }
        if let Some(ShellEvent::Refresh(page)) = event {
            let dirty = match page {
                Page::Products => self.remote.page_dirty(false),
                Page::Orders => self.remote.page_dirty(true),
                Page::TradeSales => self.remote.trade_dirty(),
                Page::Materials => self.remote.material_dirty(),
                Page::Lab => lab_dirty,
                _ => false,
            };
            if dirty {
                self.pending_refresh = Some(page);
                self.remote.hold_for_leave();
                self.lab.hold_for_leave();
            } else {
                self.refresh_page(page);
            }
        }
        if let Some(page) = self.pending_refresh.or(self.shell.pending_close()) {
            let page_dirty = match page {
                Page::Products => self.remote.page_dirty(false),
                Page::Orders => self.remote.page_dirty(true),
                Page::TradeSales => self.remote.trade_dirty(),
                Page::Materials => self.remote.material_dirty(),
                Page::Lab => lab_dirty,
                _ => false,
            };
            if self.tab_saving && !page_dirty {
                self.finish_page_intent(page);
            } else {
                if self.tab_saving && self.remote.leave_blocked().is_none() {
                    self.tab_saving = false;
                    self.tab_error = Some("仍有未保存修改，请检查编辑结果后继续保存。".into());
                }
                egui::Window::new("未保存的修改")
                    .collapsible(false)
                    .resizable(false)
                    .show(root.ctx(), |ui| {
                        ui.label(format!("{}有未保存修改", page.label()));
                        if let Some(error) = &self.tab_error {
                            ui.label(error);
                        }
                        if matches!(
                            page,
                            Page::Products | Page::Orders | Page::TradeSales | Page::Materials
                        ) {
                            if ui
                                .add_enabled(
                                    !self.tab_saving,
                                    egui::Button::new(if self.pending_refresh.is_some() {
                                        "保存并刷新"
                                    } else {
                                        "保存并关闭"
                                    }),
                                )
                                .clicked()
                            {
                                match if page == Page::Materials {
                                    self.remote.request_save_material(root.ctx())
                                } else if page == Page::TradeSales {
                                    self.remote.request_save_trade(root.ctx())
                                } else {
                                    self.remote
                                        .request_save_page(root.ctx(), page == Page::Orders)
                                } {
                                    Ok(true) => {
                                        self.finish_page_intent(page);
                                    }
                                    Ok(false) => {
                                        self.tab_saving = true;
                                        self.tab_error = None;
                                    }
                                    Err(error) => {
                                        self.tab_error = Some(error);
                                        self.tab_saving = false;
                                    }
                                }
                            }
                            if ui
                                .add_enabled(
                                    !self.tab_saving,
                                    egui::Button::new(if self.pending_refresh.is_some() {
                                        "放弃并刷新"
                                    } else {
                                        "放弃并关闭"
                                    }),
                                )
                                .clicked()
                            {
                                if page == Page::Materials {
                                    match self.remote.material_discard(root.ctx()) {
                                        Ok(true) => self.finish_page_intent(page),
                                        Ok(false) => {
                                            self.tab_saving = true;
                                            self.tab_error = None;
                                        }
                                        Err(error) => self.tab_error = Some(error),
                                    }
                                } else if page == Page::TradeSales {
                                    match self.remote.trade_discard(root.ctx()) {
                                        Ok(true) => self.finish_page_intent(page),
                                        Ok(false) => {
                                            self.tab_saving = true;
                                            self.tab_error = None;
                                        }
                                        Err(error) => self.tab_error = Some(error),
                                    }
                                } else {
                                    match self.remote.discard_page(page == Page::Orders) {
                                        Ok(()) => {
                                            self.finish_page_intent(page);
                                        }
                                        Err(error) => self.tab_error = Some(error.into()),
                                    }
                                }
                            }
                        }
                        if ui.button("返回编辑").clicked() {
                            self.shell.cancel_close();
                            self.pending_refresh = None;
                            self.shell.activate(page);
                            self.tab_saving = false;
                            self.tab_error = None;
                        }
                    });
            }
        }
        if products_open && !self.shell.is_open(Page::Products) {
            self.remote.suspend_closed_page(false);
        }
        if orders_open && !self.shell.is_open(Page::Orders) {
            self.remote.suspend_closed_page(true);
        }
        if trade_open && !self.shell.is_open(Page::TradeSales) {
            self.remote.trade_suspend();
        }
        if material_open && !self.shell.is_open(Page::Materials) {
            self.remote.material_suspend();
        }
        if let Some(storage) = frame.storage_mut() {
            self.remote.sync_views(storage);
        } else {
            self.remote.views_storage_unavailable();
        }
        let active = self.shell.active();
        if !self.shell.is_open(active) {
            egui::CentralPanel::default().show(root, |_| {});
        } else {
            root.push_id(("erp-page", active), |root| match active {
                Page::Products => {
                    egui::CentralPanel::default().show(root, |ui| self.remote.ui(ui));
                }
                Page::Orders => {
                    egui::CentralPanel::default().show(root, |ui| self.remote.orders_ui(ui));
                }
                Page::TradeSales => {
                    egui::CentralPanel::default().show(root, |ui| self.remote.trade_ui(ui));
                }
                Page::Materials => {
                    egui::CentralPanel::default().show(root, |ui| self.remote.material_ui(ui));
                }
                Page::Reports => {
                    egui::CentralPanel::default().show(root, |ui| self.reports.ui(ui));
                }
                Page::WarehouseTree => {
                    egui::CentralPanel::default().show(root, |ui| self.tree.ui(ui));
                }
                Page::Lab => self.lab.ui(root, frame),
            });
        }
        if let Some(storage) = frame.storage_mut() {
            self.remote.persist_views(storage);
            self.shell.persist(storage);
        }
        if self.leave.pending() {
            let dirty = self.remote.dirty()
                || self.lab.main.dirty()
                || self.lab.goods.dirty()
                || self.lab.remote.dirty();
            if self.leave.saving()
                && dirty
                && self.remote.leave_blocked().is_none()
                && self.lab.remote.leave_blocked().is_none()
            {
                self.leave
                    .save_failed("仍有未保存修改，请检查编辑结果后继续保存。".into());
            }
            if self.leave.saving() && !dirty {
                self.leave.resolve(Choice::Save);
                self.allow_close = true;
                root.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            } else if let Some(choice) = self.leave.dialog(root.ctx(), self.remote.leave_blocked())
            {
                match choice {
                    Choice::KeepEditing => {
                        self.discard_waiting = false;
                        self.leave.resolve(choice);
                    }
                    Choice::Discard => match self.finish_discard(root.ctx()) {
                        Ok(true) => {}
                        Ok(false) => self.discard_waiting = true,
                        Err(error) => self.leave.save_failed(error),
                    },
                    Choice::Save => {
                        let result = self
                            .remote
                            .request_save_for_leave(root.ctx())
                            .and_then(|ready| {
                                self.lab
                                    .remote
                                    .request_save_for_leave(root.ctx())
                                    .map(|lab| ready && lab)
                            })
                            .and_then(|ready| {
                                self.lab.main.save_for_leave().map(|local| ready && local)
                            })
                            .and_then(|ready| {
                                self.lab.goods.save_for_leave().map(|local| ready && local)
                            });
                        match result {
                            Ok(true) => {
                                self.leave.resolve(choice);
                                self.allow_close = true;
                                root.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            Ok(false) => self.leave.wait_for_save(),
                            Err(error) => self.leave.save_failed(error),
                        }
                    }
                }
            }
        }
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.remote.persist_views(storage);
        self.shell.persist(storage);
        self.lab.save(storage);
    }
}
