//! A local fixture host for composing the workbench modules. No business API access.
mod table;
mod detail_save;
use crate::workbench::{
    detail::{DetailEvent, DetailField, DetailPanel, DetailSaveOutcome, DetailSaveRequest},
    layout::{Region, WorkspaceLayout},
    navigation::{Navigation, NavigationEvent, PageEntry},
    subtable::SubtableTabs,
    toolbar::{ToolbarEvent, ToolbarState},
    tree::{TreeFilter, TreeNode},
};
use eframe::egui;
use erp_grid_core::{Column, ColumnId, FilterKind, RowId, ValueKind};
use std::collections::{BTreeMap, VecDeque};
use table::{Row, Table};

const NUMBER: ColumnId = ColumnId(1);
const NAME: ColumnId = ColumnId(2);
const STATUS: ColumnId = ColumnId(3);
const AMOUNT: ColumnId = ColumnId(4);
const CHANNEL: ColumnId = ColumnId(5);
const NOTE: ColumnId = ColumnId(6);
const PHONE: ColumnId = ColumnId(7);
const TRACKING: ColumnId = ColumnId(8);

struct ExamplePage {
    toolbar: ToolbarState,
    tree: TreeFilter,
    nodes: Vec<TreeNode>,
    layout: WorkspaceLayout,
    main: Table,
    goods: Table,
    logs: Table,
    detail: DetailPanel,
    persistence: crate::workbench::detail_persistence::DetailPersistence,
    record_versions: BTreeMap<RowId, u64>,
    save_job: Option<(DetailSaveRequest, f64)>,
    last_save_result: Option<(DetailSaveRequest, DetailSaveOutcome)>,
    subtabs: SubtableTabs,
    saved_logs: BTreeMap<RowId, Vec<Row>>,
    selected: Option<RowId>,
    child_parent: Option<RowId>,
    notice: Option<String>,
    delivery: Option<crate::components::Delivery>,
}
impl ExamplePage {
    fn new(stock: bool) -> Self {
        let columns = vec![
            Column::new(
                NUMBER,
                if stock {
                    "货品编码"
                } else {
                    "网店订单号"
                },
                180.0,
            ),
            Column::new(
                NAME,
                if stock {
                    "货品名称"
                } else {
                    "客户名称"
                },
                160.0,
            ),
            Column::new(STATUS, "处理状态", 100.0)
                .with_filter(FilterKind::Enum(vec!["待处理".into(), "已完成".into()])),
            Column::new(
                AMOUNT,
                if stock {
                    "库存金额"
                } else {
                    "订单金额"
                },
                100.0,
            )
            .with_filter(FilterKind::Money),
            Column::new(CHANNEL, if stock { "仓库" } else { "销售渠道" }, 120.0)
                .with_filter(FilterKind::Enum(vec!["直营".into(), "平台".into()])),
            Column::new(NOTE, "备注", 220.0),
            Column::new(PHONE, "手机号", 125.0),
            Column::new(TRACKING, "快递单号", 170.0),
        ];
        let rows = (1..=120)
            .map(|i| Row {
                id: RowId(i),
                cells: [
                    (
                        NUMBER,
                        format!("{}202609{:06}", if stock { "SP" } else { "DD" }, i),
                    ),
                    (
                        NAME,
                        format!(
                            "{} {:03}",
                            if stock {
                                "示例货品"
                            } else {
                                "示例客户"
                            },
                            i
                        ),
                    ),
                    (STATUS, if i % 3 == 0 { "已完成" } else { "待处理" }.into()),
                    (AMOUNT, format!("{}.{:02}", i * 13, i % 100)),
                    (CHANNEL, if i % 2 == 0 { "直营" } else { "平台" }.into()),
                    (NOTE, String::new()),
                    (PHONE, format!("1380000{:04}", i)),
                    (TRACKING, format!("DEMO{:012}", i)),
                ]
                .into_iter()
                .collect(),
            })
            .collect();
        let mut main = Table::new(columns, rows);
        main.columns[3].value_kind = ValueKind::Money;
        let nodes = vec![TreeNode {
            id: "all".into(),
            label: "全部".into(),
            count: Some(120),
            children: vec![
                TreeNode {
                    id: "status".into(),
                    label: "处理状态".into(),
                    count: None,
                    children: vec![leaf("pending", "待处理", 80), leaf("done", "已完成", 40)],
                },
                TreeNode {
                    id: "channel".into(),
                    label: if stock {
                        "仓库分类"
                    } else {
                        "销售渠道"
                    }
                    .into(),
                    count: None,
                    children: vec![leaf("direct", "直营", 60), leaf("platform", "平台", 60)],
                },
            ],
        }];
        let mut tree = TreeFilter {
            selected: Some("all".into()),
            ..Default::default()
        };
        tree.expanded
            .extend(["all".into(), "status".into(), "channel".into()]);
        Self {
            toolbar: ToolbarState::default(),
            tree,
            nodes,
            layout: Default::default(),
            main,
            goods: Table::new(
                vec![
                    Column::new(NUMBER, "货品编码", 150.0),
                    Column::new(NAME, "货品名称", 200.0),
                    Column::new(AMOUNT, "数量", 90.0),
                ],
                Vec::new(),
            ),
            logs: Table::new(
                vec![
                    Column::new(NUMBER, "序", 70.0),
                    Column::new(NAME, "操作", 160.0),
                    Column::new(NOTE, "内容", 300.0),
                ],
                Vec::new(),
            ),
            detail: DetailPanel::new(if stock { "local-stock-module" } else { "local-order-module" }.into()),
            persistence: crate::workbench::detail_persistence::DetailPersistence::new(if stock {"local-stock-module"} else {"local-order-module"}.into()),
            record_versions: BTreeMap::new(),
            save_job: None,
            last_save_result: None,
            subtabs: Default::default(),
            saved_logs: BTreeMap::new(),
            selected: None,
            child_parent: None,
            notice: None,
            delivery: None,
        }
    }
    fn filter(&self) -> Option<(ColumnId, &'static str)> {
        match self.tree.selected.as_deref() {
            Some("pending") => Some((STATUS, "待处理")),
            Some("done") => Some((STATUS, "已完成")),
            Some("direct") => Some((CHANNEL, "直营")),
            Some("platform") => Some((CHANNEL, "平台")),
            _ => None,
        }
    }
    fn sync_children(&mut self) {
        if self.child_parent == self.selected {
            return;
        }
        self.child_parent = self.selected;
        let goods = self
            .selected
            .map(|parent| {
                (1..=3)
                    .map(|i| Row {
                        id: RowId(parent.0 * 100 + i),
                        cells: [
                            (NUMBER, format!("SKU-{:04}-{}", parent.0, i)),
                            (NAME, format!("示例明细货品 {i}")),
                            (AMOUNT, (i + parent.0 % 5).to_string()),
                        ]
                        .into_iter()
                        .collect(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.goods.replace(goods);
        self.logs.replace(
            self.selected
                .map(|parent| {
                    self.saved_logs.get(&parent).cloned().unwrap_or_else(|| {
                        vec![Row {
                            id: RowId(parent.0 * 100),
                            cells: [
                                (NUMBER, "1".into()),
                                (NAME, "建立示例记录".into()),
                                (NOTE, "本地示例数据".into()),
                            ]
                            .into_iter()
                            .collect(),
                        }]
                    })
                })
                .unwrap_or_default(),
        );
    }
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let filter = self.filter();
        self.main.external_query_input = Some(ToolbarState::query_input_id(ui));
        self.main.prepare_query(ui.ctx(), &self.toolbar.search, filter);
        let dirty = self.detail.is_dirty();
        let count = if self.delivery.is_some() {
            0
        } else {
            self.main.targets().len()
        };
        for event in self.toolbar.show(ui, count, dirty) {
            match event {
                ToolbarEvent::SearchChanged(value) => {
                    self.main.input_changed(ui.input(|input| input.time), value.is_empty());
                }
                ToolbarEvent::Reset => {
                    self.toolbar.search.clear();
                    self.tree.selected = Some("all".into());
                    self.main.reset();
                }
                ToolbarEvent::Refresh => {
                    self.main.invalidate();
                    self.notice = None;
                }
                ToolbarEvent::Export => {
                    let ids = self.main.targets();
                    if ids.is_empty() {
                        continue;
                    }
                    let rows = self
                        .main
                        .rows
                        .iter()
                        .filter(|r| ids.contains(&r.id))
                        .map(|r| self.main.columns.iter().map(|c| r.text(c.id)).collect())
                        .collect::<Vec<_>>();
                    match crate::components::encode(
                        &self.main.columns,
                        &rows,
                        crate::components::Format::Xlsx,
                    ) {
                        Ok(bytes) => {
                            self.delivery = Some(crate::components::Delivery::start(
                                bytes,
                                crate::components::Format::Xlsx,
                            ))
                        }
                        Err(e) => self.notice = Some(e),
                    }
                }
                _ => {}
            }
        }
        if let Some(delivery) = &self.delivery {
            if let Some(result) = delivery.poll() {
                self.notice = Some(result.unwrap_or_else(|e| e));
                self.delivery = None;
            } else {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(50));
            }
        }
        if let Some(notice) = &self.notice {
            ui.horizontal(|ui| {
                ui.label(notice);
            });
        }
    }
    fn region(&mut self, region: Region, ui: &mut egui::Ui) {
        match region {
            Region::Left => {
                ui.add_enabled_ui(!self.detail.is_dirty(), |ui| {
                    self.tree.show(ui, &self.nodes);
                });
            }
            Region::Main => {
                self.toolbar(ui);
                let filter = self.filter();
                self.main.locked = self.detail.is_dirty();
                self.main.state.filtering_enabled = !self.detail.is_dirty();
                self.main.state.sorting_enabled = !self.detail.is_dirty();
                let previous_selection = self.main.state.selection.clone();
                let previous_focus = self.main.state.focus;
                self.main.show(ui, &self.toolbar.search, filter);
                if self.detail.is_dirty()
                    && self.main.state.selection.regions() != previous_selection.regions()
                {
                    self.main.state.selection = previous_selection.clone();
                    self.main.state.focus = previous_focus;
                    ui.ctx().request_repaint();
                }
                let next = self.main.selected();
                if next != self.selected {
                    if self.detail.is_dirty() {
                        self.main.state.row_selection.active = self.selected;
                        self.main.state.selection = previous_selection;
                        self.notice = Some((if self.detail.has_unresolved_save(){"原保存结果未确认，请先回查"}else{"请先保存或取消右侧修改"}).into());
                        ui.ctx().request_repaint();
                    } else {
                        self.selected = next;
                        self.sync_children();
                    }
                }
            }
            Region::Bottom => {
                self.sync_children();
                if self
                    .subtabs
                    .show(ui, &[("goods", "货品"), ("logs", "日志")])
                    == Some("logs")
                {
                    ui.push_id("logs", |ui| self.logs.show(ui, "", None));
                } else {
                    ui.push_id("goods", |ui| self.goods.show(ui, "", None));
                }
            }
            Region::Right => {
                if let Some(error)=&self.persistence.error {
                    ui.colored_label(egui::Color32::RED,error);
                    if ui.button("重新打开恢复存储").clicked(){self.persistence.reopen();}
                }
                if let Some(request)=self.persistence.show_backup(ui,&mut self.detail) {
                    let outcome=self.last_save_result.as_ref().filter(|(original,_)|original==&request).map(|(_,result)|result.clone())
                        .unwrap_or_else(||DetailSaveOutcome::Unknown("副本原结果尚未核实，副本保留".into()));
                    if let Err(error)=self.persistence.resolve_backup(&request,outcome){self.notice=Some(error);}
                }
                if let Some(request)=self.persistence.show_conflict(ui,&self.detail) {
                    let outcome=self.last_save_result.as_ref().filter(|(original,_)|original==&request).map(|(_,result)|result.clone())
                        .unwrap_or_else(||DetailSaveOutcome::Unknown("存储原结果尚未核实，当前草稿保留".into()));
                    if let Err(error)=self.persistence.resolve_conflict(&request,outcome){self.notice=Some(error);}
                }
                let fields = self.fields();
                if let Some(id) = self.selected {
                    let version=self.version(id);
                    for event in self.detail.show(ui, &id.0.to_string(), &version, &fields) {
                        match event {
                            DetailEvent::Save(_) => { ui.ctx().request_repaint(); }
                            DetailEvent::Lookup(request) => self.lookup_save(request),
                            DetailEvent::Discard => self.notice = None,
                            DetailEvent::Blocked { .. } => {
                                self.notice = Some("请先保存或取消详情修改".into())
                            }
                        }
                    }
                }
            }
        }
    }
}
fn leaf(id: &str, label: &str, count: usize) -> TreeNode {
    TreeNode {
        id: id.into(),
        label: label.into(),
        count: Some(count),
        children: Vec::new(),
    }
}

pub struct ModuleDemo {
    navigation: Navigation,
    entries: Vec<PageEntry>,
    pages: BTreeMap<String, ExamplePage>,
    close_queue: VecDeque<String>,
    closing_window: bool,
    allow_close: bool,
    leave: crate::leave_guard::LeaveGuard,
}
impl ModuleDemo {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::fonts::install(&cc.egui_ctx);
        let dark = cc
            .storage
            .and_then(|storage| storage.get_string("ui-module-demo:theme"))
            .as_deref()
            != Some("light");
        crate::workbench::theme::install(&cc.egui_ctx, dark);
        let entries = vec![
            PageEntry {
                id: "orders".into(),
                module: "销售".into(),
                group: "示例".into(),
                title: "订单工作台".into(),
            },
            PageEntry {
                id: "stock".into(),
                module: "货品".into(),
                group: "示例".into(),
                title: "库存工作台".into(),
            },
        ];
        let mut pages = BTreeMap::from([
            ("orders".into(), ExamplePage::new(false)),
            ("stock".into(), ExamplePage::new(true)),
        ]);
        if let Some(storage) = cc.storage {
            for (id, page) in &mut pages {
                if let Some(text) = storage.get_string(&format!("ui-module-demo:{id}:layout"))
                    && text.len() < 4096
                    && let Ok(mut layout) = serde_json::from_str::<WorkspaceLayout>(&text)
                {
                    layout.validate();
                    page.layout = layout;
                }
            }
        }
        let mut navigation = Navigation::default();
        navigation.open("orders");
        Self {
            navigation,
            entries,
            pages,
            close_queue: VecDeque::new(),
            closing_window: false,
            allow_close: false,
            leave: Default::default(),
        }
    }
}
impl eframe::App for ModuleDemo {
    fn ui(&mut self, root: &mut egui::Ui, frame: &mut eframe::Frame) {
        let now=root.input(|input|input.time);
        for page in self.pages.values_mut() { page.poll_durable(root.ctx(),now); page.poll_save(now); }
        if self.pages.values().any(|page|page.save_job.is_some() || (page.persistence.error.is_none() && page.persistence.pending())) {root.ctx().request_repaint_after(std::time::Duration::from_millis(33));}
        let dirty = self.pages.values().any(|p| p.detail.is_dirty() || p.persistence.pending());
        if self.closing_window && !dirty {
            self.allow_close=true;self.closing_window=false;root.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        self.leave.sync_before_unload(dirty);
        if root.input(|i| i.viewport().close_requested()) && !self.allow_close {
            if dirty {
                root.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.closing_window = true;
            } else {
                self.allow_close = true;
            }
        }
        let pages = &self.pages;
        for event in self.navigation.show(root, &self.entries, |id| {
            pages.get(id).is_some_and(|p| p.detail.is_dirty())
        }) {
            if let NavigationEvent::ThemeChanged(dark) = event {
                root.ctx().set_theme(if dark {
                    egui::Theme::Dark
                } else {
                    egui::Theme::Light
                });
                if let Some(storage) = frame.storage_mut() {
                    storage.set_string(
                        "ui-module-demo:theme",
                        if dark { "dark" } else { "light" }.into(),
                    );
                }
            }
            if let NavigationEvent::Close(id) = event
                && !self.close_queue.contains(&id)
            {
                self.close_queue.push_back(id);
            }
        }
        if let Some(id) = self.navigation.active().map(str::to_owned)
            && let Some(page) = self.pages.get_mut(&id)
        {
            root.push_id(&id, |root| {
                let mut layout = std::mem::take(&mut page.layout);
                let visible = [
                    page.toolbar.left_open,
                    page.toolbar.bottom_open,
                    page.toolbar.right_open,
                ];
                layout.show(root, visible, page, |page, region, ui| {
                    page.region(region, ui)
                });
                if visible
                    != [
                        page.toolbar.left_open,
                        page.toolbar.bottom_open,
                        page.toolbar.right_open,
                    ]
                {
                    root.ctx().request_repaint();
                }
                page.layout = layout;
            });
        }
        while let Some(id) = self.close_queue.front() {
            if self.pages.get(id).is_some_and(|p| p.detail.is_dirty() || p.persistence.pending()) {
                break;
            }
            self.navigation.commit_close(id);
            self.close_queue.pop_front();
        }
        if self.closing_window || !self.close_queue.is_empty() {
            egui::Modal::new(root.id().with("unsaved-close")).show(root.ctx(), |ui| {
                let unresolved=self.pages.values().any(|page|page.detail.has_unresolved_save() || page.persistence.pending());
                ui.label(if unresolved {"保存结果未确认"} else {"未保存修改"});
                ui.horizontal(|ui| {
                    if ui.button("保留编辑").clicked() {
                        self.close_queue.clear();
                        self.closing_window = false;
                    }
                    let discard_allowed=detail_save::can_discard_close(
                        &self.pages,self.close_queue.front().map(String::as_str),self.closing_window);
                    if ui.add_enabled(discard_allowed,egui::Button::new("放弃并关闭")).clicked() {
                        if self.closing_window {
                            self.allow_close = true;
                            root.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        } else if let Some(id) = self.close_queue.front().cloned() {
                            let discarded=self.pages.get_mut(&id).is_none_or(|page|page.detail.reset().is_ok());
                            if discarded {self.close_queue.pop_front();self.navigation.commit_close(&id);}
                        }
                        self.closing_window = false;
                    }
                });
            });
        }
        if let Some(storage) = frame.storage_mut() {
            for (id, page) in &self.pages {
                if let Ok(text) = serde_json::to_string(&page.layout) {
                    let key = format!("ui-module-demo:{id}:layout");
                    if storage.get_string(&key).as_deref() != Some(&text) {
                        storage.set_string(&key, text);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn module_demo_pages_and_child_logs_are_independent() {
        let mut first = ExamplePage::new(false);
        let second = ExamplePage::new(true);
        first.selected = Some(RowId(1));
        first.sync_children();
        let fields=first.fields();
        first.detail.bind("1","0",&fields);
        first.detail.edit_value(&NAME.0.to_string(),"修改后的客户".into()).expect("editable local name");
        let request=first.detail.begin_save().expect("local save intent");
        crate::workbench::detail_storage::authorize_fixture(&mut first.detail,&request).expect("actual persisted fixture");
        first.save(request);
        first.selected = Some(RowId(2));
        first.sync_children();
        first.selected = Some(RowId(1));
        first.sync_children();
        assert_eq!(first.logs.rows.len(), 2);
        assert_eq!(first.main.rows[0].text(NAME), "修改后的客户");
        assert_ne!(second.main.rows[0].text(NAME), "修改后的客户");
    }
}
