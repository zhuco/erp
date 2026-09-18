use egui::{Button, Ui};

/// User intents emitted by [`BatchActionPanel`]. The host owns execution and
/// identity checks; this component only renders state and reports a click.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BatchActionIntent {
    Continue,
    VerifyUnknown,
    CancelUnstarted,
    Pause,
    LocateResult,
}

/// Reusable progress and result feedback for a batch operation.
#[derive(Debug, Default)]
pub struct BatchActionPanel<'a> {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub paused: bool,
    pub storage_failed: bool,
    pub authorized: bool,
    pub can_write: bool,
    pub has_unknown: bool,
    pub has_conflict: bool,
    pub ready: bool,
    pub can_pause: bool,
    pub result_location: Option<&'a str>,
    pub locate_label: Option<&'a str>,
    pub disabled_reason: Option<&'a str>,
    pub notice: Option<&'a str>,
    pub failures: &'a [&'a str],
}

impl<'a> BatchActionPanel<'a> {
    /// Returns the retry action when the current paused batch may be resumed.
    /// Unknown items deliberately take precedence so the host can perform a
    /// signed lookup before sending anything again.
    pub fn retry_intent(&self) -> Option<BatchActionIntent> {
        (self.paused
            && !self.storage_failed
            && self.authorized
            && self.can_write
            && !self.has_conflict)
            .then_some(if self.has_unknown {
                BatchActionIntent::VerifyUnknown
            } else {
                BatchActionIntent::Continue
            })
    }

    pub fn show(&self, ui: &mut Ui) -> Option<BatchActionIntent> {
        let mut intent = None;
        ui.label(format!(
            "成功 {} · 失败 {} / {}",
            self.success, self.failed, self.total
        ));

        if !self.storage_failed && self.paused {
            let verify = self.has_unknown;
            let enabled = self.retry_intent().is_some();
            if ui
                .add_enabled(
                    enabled,
                    Button::new(if verify {
                        "核实上次操作"
                    } else {
                        "继续"
                    }),
                )
                .on_disabled_hover_text(
                    self.disabled_reason
                        .unwrap_or("当前操作暂不可继续，请先处理未完成状态"),
                )
                .clicked()
            {
                intent = Some(if verify {
                    BatchActionIntent::VerifyUnknown
                } else {
                    BatchActionIntent::Continue
                });
            }
            if self.ready && ui.button("取消未发送项").clicked() {
                intent = Some(BatchActionIntent::CancelUnstarted);
            }
        } else if !self.storage_failed && self.can_pause && ui.button("暂停").clicked() {
            intent = Some(BatchActionIntent::Pause);
        }

        if self.result_location.is_some()
            && ui.button(self.locate_label.unwrap_or("定位结果")).clicked()
        {
            intent = Some(BatchActionIntent::LocateResult);
        }
        if let Some(notice) = self.notice {
            ui.label(notice);
        }
        if !self.failures.is_empty() {
            egui::ScrollArea::vertical().max_height(120.0).show_rows(
                ui,
                ui.text_style_height(&egui::TextStyle::Body),
                self.failures.len(),
                |ui, range| {
                    for index in range {
                        ui.add(egui::Label::new(self.failures[index]).truncate())
                            .on_hover_text(self.failures[index]);
                    }
                },
            );
        }
        intent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_batch_retry_requires_authorization_and_uses_verify() {
        let unauthorized = BatchActionPanel {
            paused: true,
            has_unknown: true,
            can_write: true,
            ..Default::default()
        };
        assert_eq!(unauthorized.retry_intent(), None);

        let authorized = BatchActionPanel {
            paused: true,
            has_unknown: true,
            authorized: true,
            can_write: true,
            ..Default::default()
        };
        assert_eq!(
            authorized.retry_intent(),
            Some(BatchActionIntent::VerifyUnknown)
        );
    }

    #[test]
    fn click_continue_emits_continue_intent() {
        let ctx = egui::Context::default();
        let panel = BatchActionPanel {
            paused: true,
            authorized: true,
            can_write: true,
            ..Default::default()
        };
        let mut output = None;
        let input = |events| egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(400.0, 200.0),
            )),
            events,
            ..Default::default()
        };
        let mut frame = ctx.run_ui(input(vec![]), |root| {
            egui::CentralPanel::default().show(root, |ui| {
                let _ = panel.show(ui);
            });
        });
        frame.textures_delta.clear();
        let mut frame = ctx.run_ui(
            input(vec![egui::Event::PointerButton {
                pos: egui::Pos2::new(12.0, 40.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }]),
            |root| {
                egui::CentralPanel::default().show(root, |ui| {
                    let _ = panel.show(ui);
                });
            },
        );
        frame.textures_delta.clear();
        let mut frame = ctx.run_ui(
            input(vec![egui::Event::PointerButton {
                pos: egui::Pos2::new(12.0, 40.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }]),
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    output = panel.show(ui);
                });
            },
        );
        frame.textures_delta.clear();
        assert_eq!(output, Some(BatchActionIntent::Continue));
    }
}
