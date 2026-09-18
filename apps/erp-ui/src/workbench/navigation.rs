//! Generic workbench navigation. Hosts own page state and close confirmation.

use eframe::egui;

const MAX_TABS: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageEntry {
    pub id: String,
    pub module: String,
    pub group: String,
    pub title: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavigationEvent {
    Open(String),
    Activate(String),
    Close(String),
    ThemeChanged(bool),
}

#[derive(Clone, Debug, Default)]
pub struct Navigation {
    tabs: Vec<String>,
    active: Option<String>,
    search: String,
}

impl Navigation {
    pub fn active(&self) -> Option<&str> {
        self.active.as_deref()
    }

    pub fn tab_ids(&self) -> &[String] {
        &self.tabs
    }

    pub fn open(&mut self, id: &str) -> bool {
        if id.is_empty() || self.tabs.iter().any(|tab| tab == id) || self.tabs.len() >= MAX_TABS {
            return false;
        }
        self.tabs.push(id.to_owned());
        self.active = Some(id.to_owned());
        true
    }

    /// Commits a close after the host has checked dirty state.
    pub fn commit_close(&mut self, id: &str) -> bool {
        let Some(index) = self.tabs.iter().position(|tab| tab == id) else {
            return false;
        };
        self.tabs.remove(index);
        if self.active.as_deref() == Some(id) {
            self.active = self
                .tabs
                .get(index.min(self.tabs.len().saturating_sub(1)))
                .cloned();
        }
        true
    }

    pub fn show<F>(
        &mut self,
        ui: &mut egui::Ui,
        entries: &[PageEntry],
        dirty: F,
    ) -> Vec<NavigationEvent>
    where
        F: Fn(&str) -> bool,
    {
        let mut events = Vec::new();
        egui::Panel::top(ui.id().with("navigation-menu"))
            .exact_size(40.0)
            .frame(
                egui::Frame::new()
                    .fill(ui.visuals().window_fill)
                    .inner_margin(6.0),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("易格")
                            .strong()
                            .size(16.0)
                            .color(ui.visuals().hyperlink_color),
                    );
                    ui.separator();
                    let mut modules = Vec::<String>::new();
                    for entry in entries.iter().filter(|entry| !entry.id.is_empty()) {
                        if !modules.iter().any(|module| module == &entry.module) {
                            modules.push(entry.module.clone());
                        }
                    }
                    for module in modules {
                        ui.menu_button(&module, |ui| {
                            for entry in entries.iter().filter(|entry| entry.module == module) {
                                if ui
                                    .button(format!("{} · {}", entry.group, entry.title))
                                    .clicked()
                                {
                                    if self.open(&entry.id) {
                                        events.push(NavigationEvent::Open(entry.id.clone()));
                                    } else if self.tabs.iter().any(|tab| tab == &entry.id) {
                                        self.active = Some(entry.id.clone());
                                        events.push(NavigationEvent::Activate(entry.id.clone()));
                                    }
                                    ui.close();
                                }
                            }
                        });
                    }
                    ui.add_space(8.0);
                    ui.add_sized(
                        [180.0, 22.0],
                        egui::TextEdit::singleline(&mut self.search).hint_text("搜索功能…"),
                    );
                    let needle = self.search.trim().to_lowercase();
                    if !needle.is_empty() {
                        ui.menu_button("结果", |ui| {
                            for entry in entries.iter().filter(|entry| {
                                entry.title.to_lowercase().contains(&needle)
                                    || entry.group.to_lowercase().contains(&needle)
                            }) {
                                if ui.button(&entry.title).clicked() {
                                    if self.open(&entry.id) {
                                        events.push(NavigationEvent::Open(entry.id.clone()));
                                    } else if self.tabs.iter().any(|tab| tab == &entry.id) {
                                        self.active = Some(entry.id.clone());
                                        events.push(NavigationEvent::Activate(entry.id.clone()));
                                    }
                                    ui.close();
                                }
                            }
                        });
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let dark = ui.visuals().dark_mode;
                        if ui
                            .button(if dark { "浅色" } else { "深色" })
                            .on_hover_text("切换外观，保留当前页面状态")
                            .clicked()
                        {
                            events.push(NavigationEvent::ThemeChanged(!dark));
                        }
                    });
                });
            });

        egui::Panel::top(ui.id().with("navigation-tabs"))
            .exact_size(32.0)
            .show(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("navigation-tabs-scroll")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for id in self.tabs.clone() {
                                ui.push_id(&id, |ui| {
                                    let title = entries
                                        .iter()
                                        .find(|entry| entry.id == id)
                                        .map_or(id.as_str(), |entry| entry.title.as_str());
                                    let response = ui.selectable_label(
                                        self.active.as_deref() == Some(&id),
                                        title,
                                    );
                                    response.context_menu(|ui| {
                                        let close_label = if dirty(&id) {
                                            "关闭当前（需确认）"
                                        } else {
                                            "关闭当前"
                                        };
                                        if ui.button(close_label).clicked() {
                                            events.push(NavigationEvent::Close(id.clone()));
                                            ui.close();
                                        }
                                        if ui.button("关闭其他").clicked() {
                                            for other in
                                                self.tabs.iter().filter(|other| *other != &id)
                                            {
                                                events.push(NavigationEvent::Close(other.clone()));
                                            }
                                            ui.close();
                                        }
                                        if ui.button("关闭全部").clicked() {
                                            for other in &self.tabs {
                                                events.push(NavigationEvent::Close(other.clone()));
                                            }
                                            ui.close();
                                        }
                                    });
                                    if response.clicked() && self.active.as_deref() != Some(&id) {
                                        self.active = Some(id.clone());
                                        events.push(NavigationEvent::Activate(id.clone()));
                                    }
                                    if ui
                                        .add(egui::Button::new("×").frame(false))
                                        .on_hover_text("关闭标签")
                                        .clicked()
                                    {
                                        events.push(NavigationEvent::Close(id.clone()));
                                    }
                                });
                            }
                        });
                    });
            });

        let (previous, next) = ui.input(|input| {
            (
                input.modifiers.ctrl && input.modifiers.shift && input.key_pressed(egui::Key::Tab),
                input.modifiers.ctrl && !input.modifiers.shift && input.key_pressed(egui::Key::Tab),
            )
        });
        if (previous || next) && self.tabs.len() > 1 {
            ui.input_mut(|input| {
                input.consume_key(
                    if previous {
                        egui::Modifiers::CTRL | egui::Modifiers::SHIFT
                    } else {
                        egui::Modifiers::CTRL
                    },
                    egui::Key::Tab,
                );
            });
            let index = self
                .active
                .as_ref()
                .and_then(|id| self.tabs.iter().position(|tab| tab == id))
                .unwrap_or(0);
            let next_index = if previous {
                (index + self.tabs.len() - 1) % self.tabs.len()
            } else {
                (index + 1) % self.tabs.len()
            };
            let id = self.tabs[next_index].clone();
            self.active = Some(id.clone());
            events.push(NavigationEvent::Activate(id));
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_is_deduplicated_and_bounded() {
        let mut navigation = Navigation::default();
        assert!(navigation.open("a"));
        assert!(!navigation.open("a"));
        for index in 0..MAX_TABS {
            let _ = navigation.open(&format!("{index}"));
        }
        assert_eq!(navigation.tab_ids().len(), MAX_TABS);
        assert!(!navigation.open("overflow"));
    }

    #[test]
    fn close_requires_commit_and_keeps_active_valid() {
        let mut navigation = Navigation::default();
        navigation.open("a");
        navigation.open("b");
        assert_eq!(navigation.active(), Some("b"));
        assert!(!navigation.commit_close("missing"));
        assert!(navigation.commit_close("b"));
        assert_eq!(navigation.active(), Some("a"));
    }
}
