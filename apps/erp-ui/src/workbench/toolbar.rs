use eframe::egui;

#[derive(Clone, Debug)]
pub struct ToolbarState {
    pub search: String,
    pub left_open: bool,
    pub right_open: bool,
    pub bottom_open: bool,
}

impl Default for ToolbarState {
    fn default() -> Self {
        Self {
            search: String::new(),
            left_open: true,
            right_open: true,
            bottom_open: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolbarEvent {
    SearchChanged(String),
    Reset,
    Refresh,
    Export,
    ToggleLeft(bool),
    ToggleRight(bool),
    ToggleBottom(bool),
}

impl ToolbarState {
    /// Search is intentionally emitted immediately; the host owns IME and
    /// debounce policy. Dirty state disables actions that would replace data.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        selected_count: usize,
        dirty: bool,
    ) -> Vec<ToolbarEvent> {
        let mut events = Vec::new();
        let height = ui.spacing().interact_size.y.max(24.0);
        let width = ui.available_width();
        // Fixed edge groups leave the search centered in the remaining space.
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        let left_width = 100.0;
        let right_width = 142.0;
        let left = egui::Rect::from_min_size(rect.min, egui::vec2(left_width, height));
        let right = egui::Rect::from_min_max(
            egui::pos2(
                (rect.right() - right_width).max(rect.left() + left_width),
                rect.top(),
            ),
            rect.max,
        );
        let center = egui::Rect::from_min_max(
            egui::pos2(left.right() + 8.0, rect.top()),
            egui::pos2((right.left() - 8.0).max(left.right() + 8.0), rect.bottom()),
        );
        ui.scope_builder(
            egui::UiBuilder::new().id_salt("operations").max_rect(left),
            |ui| {
                ui.horizontal(|ui| {
                    if ui.add_enabled(!dirty, egui::Button::new("重置")).clicked() {
                        events.push(ToolbarEvent::Reset);
                    }
                    if ui.add_enabled(!dirty, egui::Button::new("刷新")).clicked() {
                        events.push(ToolbarEvent::Refresh);
                    }
                });
            },
        );
        let search_width = center.width().clamp(0.0, 420.0);
        let search_rect =
            egui::Rect::from_center_size(center.center(), egui::vec2(search_width, height));
        ui.scope_builder(
            egui::UiBuilder::new()
                .id_salt("search")
                .max_rect(search_rect),
            |ui| {
                ui.horizontal(|ui| {
                    let changed = ui
                        .add_enabled(
                            !dirty,
                            egui::TextEdit::singleline(&mut self.search)
                                .desired_width((search_width - 32.0).max(0.0))
                                .hint_text("手机号 / 姓名 / 快递单号 / 平台单号"),
                        )
                        .on_hover_text("输入手机号、姓名、快递单号或平台单号，自动查询")
                        .changed();
                    let cleared = ui
                        .add_enabled(!dirty && !self.search.is_empty(), egui::Button::new("×"))
                        .on_hover_text("清除搜索文本")
                        .clicked();
                    if cleared {
                        self.search.clear();
                    }
                    if changed || cleared {
                        events.push(ToolbarEvent::SearchChanged(self.search.clone()));
                    }
                });
            },
        );
        ui.scope_builder(
            egui::UiBuilder::new().id_salt("utilities").max_rect(right),
            |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(selected_count > 0 && !dirty, egui::Button::new("导出"))
                        .on_hover_text("导出勾选记录；未勾选时导出当前行")
                        .clicked()
                    {
                        events.push(ToolbarEvent::Export);
                    }
                    ui.menu_button("帮助", |ui| {
                        ui.label("搜索：手机号、姓名、快递单号、平台单号");
                        ui.label("表头复选框：左键全选/取消，右键反选当页");
                        ui.label("拖选单元格复制；右键表头设置列");
                    });
                    let settings = ui
                        .menu_button("  ", |ui| {
                            if ui.checkbox(&mut self.left_open, "筛选").changed() {
                                events.push(ToolbarEvent::ToggleLeft(self.left_open));
                            }
                            if ui.checkbox(&mut self.right_open, "详情").changed() {
                                events.push(ToolbarEvent::ToggleRight(self.right_open));
                            }
                            if ui.checkbox(&mut self.bottom_open, "子表").changed() {
                                events.push(ToolbarEvent::ToggleBottom(self.bottom_open));
                            }
                        })
                        .response
                        .on_hover_text("面板设置");
                    let center = settings.rect.center();
                    let stroke = egui::Stroke::new(1.4, ui.visuals().text_color());
                    ui.painter().circle_stroke(center, 5.0, stroke);
                    ui.painter().circle_stroke(center, 1.5, stroke);
                    for spoke in 0..8 {
                        let direction =
                            egui::Vec2::angled(spoke as f32 * std::f32::consts::TAU / 8.0);
                        ui.painter().line_segment(
                            [center + direction * 5.0, center + direction * 8.0],
                            stroke,
                        );
                    }
                });
            },
        );
        events
    }
}
