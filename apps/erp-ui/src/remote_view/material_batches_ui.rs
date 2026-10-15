//! Actual scoped batch identities; a quote never substitutes for commit-time stock checks.
use super::material_documents::MaterialDocuments;
use super::material_model;
use eframe::egui;
use erp_grid_core::RowId;
use erp_ui_egui::DataGrid;
fn quantity(value: &Option<String>) -> String {
    value
        .as_ref()
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|n| *n >= 0)
        .map(material_model::display)
        .unwrap_or("—".into())
}
impl MaterialDocuments {
    pub(super) fn batch_picker(&mut self, ui: &mut egui::Ui) {
        let Some(line_id) = self.batch_line else {
            return;
        };
        let mut open = true;
        let mut chosen = None;
        egui::Window::new("选择库存批次")
            .open(&mut open)
            .default_size(egui::vec2(950.0, 380.0))
            .show(ui.ctx(), |ui| {
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut self.batch_search)
                            .hint_text("批号")
                            .char_limit(128),
                    )
                    .changed()
                {
                    self.suspend_reads();
                    self.batch_offset = 0;
                    self.batch_page = None;
                    self.batch_deadline = Some(ui.input(|i| i.time) + 0.25);
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(250));
                }
                if let Some(page) = &self.batch_page {
                    if page.needs_reconciliation {
                        ui.colored_label(
                            ui.visuals().warn_fg_color,
                            "完整批次与仓库精确量需核对，不能选择",
                        );
                    }
                    ui.label(format!(
                        "仓库可用：{} · {}",
                        quantity(&page.warehouse_available_micros),
                        if page.batch_enabled {
                            "批次管理"
                        } else {
                            "实际批次库存"
                        }
                    ));
                }
                let rows = self
                    .batch_page
                    .as_ref()
                    .map(|p| p.data.as_slice())
                    .unwrap_or(&[]);
                let height = (ui.available_height() - 38.0).max(80.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        DataGrid {
                            id: ui.id().with(("material-batch-options", line_id)),
                            columns: &mut self.batch_columns,
                            rows: rows.len(),
                            state: &mut self.batch_state,
                            row_id: Some(&|i| RowId(rows[i].batch_id.as_u128())),
                        }
                        .show(ui, |i, c| match c.0 {
                            1 => rows[i].batch_no.clone().unwrap_or("—".into()),
                            2 => match rows[i].state.as_str() {
                                "READY" => "可选",
                                "EXPIRED" => "已过期",
                                "EMPTY" => "无可用量",
                                _ => "待核对",
                            }
                            .into(),
                            3 => rows[i].expire_at.clone().unwrap_or("—".into()),
                            4 => quantity(&rows[i].on_hand_micros),
                            5 => quantity(&rows[i].reserved_micros),
                            6 => quantity(&rows[i].available_micros),
                            _ => String::new(),
                        });
                    },
                );
                let ready =
                    self.reading.is_none() && self.batch_deadline.is_none() && !self.composing;
                let total = self.batch_page.as_ref().map_or(0, |p| p.total);
                ui.horizontal(|ui| {
                    ui.label(format!("共{total}条"));
                    if ui
                        .add_enabled(ready && self.batch_offset > 0, egui::Button::new("上一页"))
                        .clicked()
                    {
                        self.batch_offset = self.batch_offset.saturating_sub(500);
                        self.batch_page = None;
                        self.batch_deadline = Some(0.0);
                    }
                    if ui
                        .add_enabled(
                            ready && self.batch_offset + 500 < total,
                            egui::Button::new("下一页"),
                        )
                        .clicked()
                    {
                        self.batch_offset += 500;
                        self.batch_page = None;
                        self.batch_deadline = Some(0.0);
                    }
                    let selectable = self.batch_state.row_selection.active.is_some_and(|id| {
                        self.batch_page.as_ref().is_some_and(|p| {
                            !p.needs_reconciliation
                                && p.data
                                    .iter()
                                    .any(|b| b.batch_id.as_u128() == id.0 && b.state == "READY")
                        })
                    });
                    if ui
                        .add_enabled(
                            ready && self.editable() && selectable,
                            egui::Button::new("采用所选批次"),
                        )
                        .clicked()
                    {
                        chosen = self.batch_state.row_selection.active;
                    }
                    if self.reading.is_some() {
                        ui.spinner();
                    }
                });
            });
        if !open {
            self.batch_line = None;
            self.batch_page = None;
            self.suspend_reads();
        }
        if let Some(id) = chosen {
            let result = (|| {
                self.commit_editor()?;
                let page = self.batch_page.as_ref().ok_or("批次查询已变化")?;
                let row = page
                    .data
                    .iter()
                    .find(|b| b.batch_id.as_u128() == id.0 && b.state == "READY")
                    .ok_or("批次不可选")?;
                let line = self
                    .draft
                    .lines
                    .iter_mut()
                    .find(|l| l.line_id == line_id)
                    .ok_or("材料行已变化")?;
                if page.warehouse_id != line.warehouse_id
                    || page.sku_id != line.sku_id
                    || !matches!(line.kind.as_str(), "ISSUE" | "SUPPLEMENT")
                {
                    return Err("批次与材料行维度不一致".into());
                }
                let name = row.batch_no.clone().ok_or("真实批号缺失")?;
                line.batch_id = Some(row.batch_id);
                self.draft.batch_labels.insert(row.batch_id, name);
                Ok(())
            })();
            match result {
                Ok(()) => {
                    self.batch_line = None;
                    self.batch_page = None;
                    self.changed();
                }
                Err(error) => self.notice = Some(error),
            }
        }
    }
}
