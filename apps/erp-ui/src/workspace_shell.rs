//! Top navigation and tab lifecycle for the ERP work area.
use eframe::egui;

#[path = "page_registry.rs"]
mod page_registry;
pub(crate) use page_registry::{PAGES, spec};

const MAX_OPEN_TABS: usize = 16;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Page {
    Products,
    Orders,
    Reports,
    WarehouseTree,
    Lab,
}
impl Page {
    pub(crate) fn label(self) -> &'static str {
        page_registry::spec(self).name
    }
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::Products => "products",
            Self::Orders => "orders",
            Self::Reports => "reports",
            Self::WarehouseTree => "warehouse-tree",
            Self::Lab => "lab",
        }
    }
    fn from_id(id: &str) -> Option<Self> {
        PAGES.iter().find(|p| p.page.id() == id).map(|p| p.page)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShellEvent {
    Activate(Page),
    Close(Page),
    Refresh(Page),
}

#[derive(Debug)]
pub(crate) struct Shell {
    open: Vec<Page>,
    active: Page,
    pending_close: Option<Page>,
    close_queue: Vec<Page>,
    search: String,
    scope: String,
    tabs_dirty: bool,
}
impl Default for Shell {
    fn default() -> Self {
        Self {
            open: vec![Page::Products],
            active: Page::Products,
            pending_close: None,
            close_queue: Vec::new(),
            search: String::new(),
            scope: String::new(),
            tabs_dirty: false,
        }
    }
}
impl Shell {
    pub(crate) fn active(&self) -> Page {
        self.active
    }
    pub(crate) fn is_open(&self, page: Page) -> bool {
        self.open.contains(&page)
    }
    pub(crate) fn activate(&mut self, page: Page) -> bool {
        if self.pending_close.is_some() {
            return false;
        }
        let was_open = self.is_open(page);
        if !was_open {
            if self.open.len() >= MAX_OPEN_TABS {
                return false;
            }
            self.open.push(page);
        }
        let changed = self.active != page || !was_open;
        self.active = page;
        self.pending_close = None;
        self.tabs_dirty |= changed;
        changed
    }
    pub(crate) fn pending_close(&self) -> Option<Page> {
        self.pending_close
    }
    pub(crate) fn commit_close(&mut self, page: Page) -> bool {
        let Some(index) = self.open.iter().position(|p| *p == page) else {
            self.pending_close = None;
            return false;
        };
        self.open.remove(index);
        self.tabs_dirty = true;
        if self.active == page {
            self.active = self
                .open
                .get(index.saturating_sub(1))
                .copied()
                .or_else(|| self.open.first().copied())
                .unwrap_or(Page::Products);
        }
        self.pending_close = None;
        true
    }
    pub(crate) fn cancel_close(&mut self) {
        self.pending_close = None;
        self.close_queue.clear();
    }

    /// Restores page ids for the caller's scope. The storage value has no credentials.
    pub(crate) fn sync(&mut self, storage: &dyn eframe::Storage, scope: Option<&str>) {
        let scope = scope.unwrap_or("");
        if self.scope == scope {
            return;
        }
        self.scope = scope.to_owned();
        self.pending_close = None;
        self.close_queue.clear();
        let key = format!("erp.workspace.tabs.{scope}");
        let pages = if scope.is_empty() {
            String::new()
        } else {
            storage
                .get_string(&key)
                .filter(|value| value.len() <= 4096)
                .unwrap_or_default()
        };
        let (active, ids) = pages.split_once('|').unwrap_or(("", pages.as_str()));
        self.open = ids
            .split(',')
            .filter_map(Page::from_id)
            .filter(|p| spec(*p).connected)
            .take(MAX_OPEN_TABS)
            .collect();
        let mut seen = std::collections::HashSet::new();
        self.open.retain(|page| seen.insert(*page));
        if self.open.is_empty() {
            self.open.push(Page::Products);
        }
        self.active = self.open[0];
        if let Some(page) = Page::from_id(active).filter(|p| self.open.contains(p)) {
            self.active = page;
        }
        self.tabs_dirty = false;
    }
    /// Persists stable page ids under the supplied scope, never identity or tokens.
    pub(crate) fn persist(&mut self, storage: &mut dyn eframe::Storage) {
        if self.scope.is_empty() || !self.tabs_dirty {
            return;
        }
        let key = format!("erp.workspace.tabs.{}", self.scope);
        let value = format!(
            "{}|{}",
            self.active.id(),
            self.open
                .iter()
                .map(|p| p.id())
                .collect::<Vec<_>>()
                .join(",")
        );
        storage.set_string(&key, value);
        self.tabs_dirty = false;
    }
    fn request_close<F>(&mut self, page: Page, dirty: &F) -> ShellEvent
    where
        F: Fn(Page) -> bool,
    {
        if dirty(page) {
            self.pending_close = Some(page);
        } else {
            self.commit_close(page);
        }
        ShellEvent::Close(page)
    }
    fn advance_close_queue<F>(&mut self, dirty: &F) -> Option<ShellEvent>
    where
        F: Fn(Page) -> bool,
    {
        if self.pending_close.is_some() {
            return None;
        }
        while let Some(page) = self.close_queue.pop() {
            if dirty(page) {
                self.pending_close = Some(page);
                return Some(ShellEvent::Close(page));
            }
            self.commit_close(page);
        }
        None
    }

    pub(crate) fn show<F>(&mut self, root: &mut egui::Ui, dirty: F) -> Option<ShellEvent>
    where
        F: Fn(Page) -> bool,
    {
        let mut event = self.advance_close_queue(&dirty);
        egui::Panel::top("erp-brand")
            .exact_size(36.0)
            .frame(
                egui::Frame::new()
                    .fill(crate::erp_theme::PRIMARY)
                    .inner_margin(6.0),
            )
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("易格 ERP")
                            .color(egui::Color32::WHITE)
                            .size(15.0),
                    );
                    ui.separator();
                    for module in ["货品", "销售", "开发工具"] {
                        ui.menu_button(module, |ui| {
                            for item in PAGES.iter().filter(|p| p.module == module && p.connected) {
                                if ui
                                    .button(format!("{} · {}", item.group, item.name))
                                    .clicked()
                                    && self.activate(item.page)
                                {
                                    event = Some(ShellEvent::Activate(item.page));
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
                    if !self.search.trim().is_empty() {
                        let needle = self.search.to_lowercase();
                        ui.menu_button("结果", |ui| {
                            for item in PAGES
                                .iter()
                                .filter(|p| p.connected && p.name.to_lowercase().contains(&needle))
                            {
                                if ui.button(item.name).clicked() && self.activate(item.page) {
                                    event = Some(ShellEvent::Activate(item.page));
                                    ui.close();
                                }
                            }
                        });
                    }
                });
            });
        egui::Panel::top("erp-workspace-tabs")
            .exact_size(30.0)
            .show(root, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("erp-tabs-scroll")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for page in self.open.clone() {
                                let response =
                                    ui.selectable_label(self.active == page, spec(page).name);
                                response.context_menu(|ui| {
                                    if page != Page::Lab && ui.button("刷新").clicked() {
                                        event = Some(ShellEvent::Refresh(page));
                                    }
                                    if ui.button("关闭当前").clicked() {
                                        event = Some(self.request_close(page, &dirty));
                                    }
                                    if ui.button("关闭其他").clicked() {
                                        self.close_queue = self
                                            .open
                                            .iter()
                                            .copied()
                                            .filter(|p| *p != page)
                                            .collect();
                                        if let Some(next) = self.close_queue.pop() {
                                            event = Some(self.request_close(next, &dirty));
                                        }
                                    }
                                    if ui.button("关闭全部").clicked() {
                                        self.close_queue = self.open.clone();
                                        if let Some(next) = self.close_queue.pop() {
                                            event = Some(self.request_close(next, &dirty));
                                        }
                                    }
                                });
                                if response.clicked() && self.activate(page) {
                                    event = Some(ShellEvent::Activate(page));
                                }
                                if ui.small_button("×").on_hover_text("关闭标签").clicked() {
                                    event = Some(self.request_close(page, &dirty));
                                }
                            }
                        });
                    });
            });
        let (previous, next) = root.input(|i| {
            (
                i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Tab),
                i.modifiers.ctrl && i.key_pressed(egui::Key::Tab),
            )
        });
        if (previous || next)
            && let Some(index) = self.open.iter().position(|p| *p == self.active)
        {
            let step = if previous { self.open.len() - 1 } else { 1 };
            let page = self.open[(index + step) % self.open.len()];
            if self.activate(page) {
                event = Some(ShellEvent::Activate(page));
            }
        }
        event
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_is_unique_and_connected() {
        let mut names = PAGES.iter().map(|p| p.name).collect::<Vec<_>>();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), PAGES.len());
        assert!(
            PAGES
                .iter()
                .all(|p| p.connected && !p.permission.is_empty())
        );
    }
    #[test]
    fn close_queue_waits_for_dirty_commit() {
        let mut shell = Shell::default();
        shell.activate(Page::Orders);
        shell.close_queue = vec![Page::Products, Page::Orders];
        let event = shell.advance_close_queue(&|page| page == Page::Orders);
        assert_eq!(event, Some(ShellEvent::Close(Page::Orders)));
        assert_eq!(shell.pending_close(), Some(Page::Orders));
        shell.commit_close(Page::Orders);
        assert_eq!(shell.pending_close(), None);
    }
    #[test]
    fn scope_change_is_the_restore_boundary() {
        let shell = Shell::default();
        assert!(shell.scope.is_empty());
    }
}
