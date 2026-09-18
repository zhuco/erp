use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    Left,
    Main,
    Bottom,
    Right,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceLayout {
    pub left_width: f32,
    pub right_width: f32,
    pub bottom_height: f32,
}
impl Default for WorkspaceLayout {
    fn default() -> Self {
        Self {
            left_width: 200.0,
            right_width: 280.0,
            bottom_height: 230.0,
        }
    }
}
impl WorkspaceLayout {
    pub fn validate(&mut self) {
        if !self.left_width.is_finite()
            || !self.right_width.is_finite()
            || !self.bottom_height.is_finite()
        {
            *self = Self::default();
        }
        self.left_width = self.left_width.clamp(140.0, 420.0);
        self.right_width = self.right_width.clamp(220.0, 520.0);
        self.bottom_height = self.bottom_height.clamp(100.0, 640.0);
    }
    pub fn show<C>(
        &mut self,
        ui: &mut egui::Ui,
        visible: [bool; 3],
        state: &mut C,
        mut draw: impl FnMut(&mut C, Region, &mut egui::Ui),
    ) {
        self.validate();
        let width = ui.available_width();
        if visible[0] {
            let panel = egui::Panel::left(ui.id().with("workspace-left"))
                .resizable(true)
                .exact_size(self.left_width.min((width * 0.3).max(140.0)))
                .size_range(140.0..=420.0)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    egui::ScrollArea::vertical().show(ui, |ui| draw(state, Region::Left, ui));
                });
            if panel.response.rect.width().is_finite() {
                self.left_width = panel.response.rect.width();
            }
        }
        if visible[2] {
            let room = (ui.available_width() - 300.0).max(220.0);
            let panel = egui::Panel::right(ui.id().with("workspace-right"))
                .resizable(true)
                .exact_size(self.right_width.min(room))
                .size_range(220.0..=520.0)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    egui::ScrollArea::vertical().show(ui, |ui| draw(state, Region::Right, ui));
                });
            if panel.response.rect.width().is_finite() {
                self.right_width = panel.response.rect.width();
            }
        }
        if visible[1] {
            let max = (ui.available_height() - 180.0).max(100.0);
            let panel = egui::Panel::bottom(ui.id().with("workspace-bottom"))
                .resizable(true)
                .exact_size(self.bottom_height.min(max))
                .size_range(100.0..=max)
                .show(ui, |ui| {
                    ui.set_min_height(ui.available_height());
                    draw(state, Region::Bottom, ui);
                });
            if panel.response.rect.height().is_finite() {
                self.bottom_height = panel.response.rect.height();
            }
        }
        egui::CentralPanel::default().show(ui, |ui| draw(state, Region::Main, ui));
    }
}
