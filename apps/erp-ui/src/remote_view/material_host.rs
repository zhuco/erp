//! Material lifecycle participates in the same authenticated host and leave guards.
use super::RemoteProductView;
use crate::remote_actions::ActionScope;
use eframe::egui;
impl RemoteProductView {
    pub(crate) fn material_dirty(&self) -> bool {
        self.material_documents.dirty()
    }
    pub(crate) fn material_discard(&mut self, ctx: &egui::Context) -> Result<bool, String> {
        self.material_documents.discard(ctx)
    }
    pub(crate) fn material_refresh(&mut self) {
        self.material_documents.refresh();
    }
    pub(crate) fn material_suspend(&mut self) {
        self.material_documents.suspend_reads();
    }
    pub(super) fn tick_material(&mut self, ctx: &egui::Context) {
        if let (Some(session), Some(actor)) = (&self.session, self.actions.actor())
            && web_time::Instant::now() < session.expires
        {
            match ActionScope::new(&session.client, session.tenant, actor) {
                Ok(scope) => self.material_documents.tick(ctx, session, scope),
                Err(e) => self.notice = Some(e),
            }
        }
    }
    pub(crate) fn request_save_material(&mut self, ctx: &egui::Context) -> Result<bool, String> {
        self.tick_material(ctx);
        let session = self
            .session
            .as_ref()
            .filter(|s| web_time::Instant::now() < s.expires)
            .ok_or("登录后保存材料单")?;
        let actor = self.actions.actor().ok_or("认证账号未就绪")?;
        let scope = ActionScope::new(&session.client, session.tenant, actor)?;
        if self.material_documents.scope.as_ref() != Some(&scope) {
            return Err("返回原账号保存材料草稿".into());
        }
        self.material_documents.save_for_leave()
    }
    pub(crate) fn material_ui(&mut self, ui: &mut egui::Ui) {
        self.tick(ui.ctx());
        self.tick_material(ui.ctx());
        if self
            .session
            .as_ref()
            .is_none_or(|s| web_time::Instant::now() >= s.expires)
        {
            self.ui_mode(ui, false);
            return;
        }
        let (Some(session), Some(actor)) = (&self.session, self.actions.actor()) else {
            ui.spinner();
            ui.label("读取账号");
            return;
        };
        match ActionScope::new(&session.client, session.tenant, actor) {
            Ok(scope) => self.material_documents.ui(ui, session, &scope),
            Err(e) => {
                ui.label(e);
            }
        }
    }
}
