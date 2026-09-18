use std::collections::{BTreeMap, BTreeSet};

use eframe::egui;
use erp_grid_core::{Column, ColumnId, LoadTicket, RowId, TreeModel, TreeNode};
use erp_ui_egui::{DataGrid, GridState, TreeRowVisual};

const LOAD_DELAY_SECONDS: f64 = 0.150;
const ROOTS: [(u128, &str); 4] = [(1, "华北仓"), (2, "华东仓"), (3, "华南仓"), (4, "西南仓")];

#[derive(Clone)]
struct Record {
    name: String,
    kind: &'static str,
    code: String,
    status: &'static str,
}

struct PendingLoad {
    ticket: LoadTicket,
    due: f64,
    result: Result<Vec<TreeNode>, String>,
    records: Vec<(RowId, Record)>,
}

pub struct TreeView {
    model: TreeModel,
    state: GridState,
    columns: Vec<Column>,
    records: BTreeMap<RowId, Record>,
    pending: Vec<PendingLoad>,
    failed_once: BTreeSet<RowId>,
    visible: Vec<erp_grid_core::TreeRow>,
    visible_dirty: bool,
    view_generation: u64,
    error: Option<String>,
}

impl Default for TreeView {
    fn default() -> Self {
        let mut state = GridState::default();
        state.read_only = true;
        state.sorting_enabled = false;
        state.filtering_enabled = false;
        state.quick_search_open = false;
        let columns = vec![
            Column::new(ColumnId(0), "名称", 260.0),
            Column::new(ColumnId(1), "层级", 110.0),
            Column::new(ColumnId(2), "编码", 180.0),
            Column::new(ColumnId(3), "状态", 110.0),
        ];
        let mut view = Self {
            model: TreeModel::new(),
            state,
            columns,
            records: BTreeMap::new(),
            pending: Vec::new(),
            failed_once: BTreeSet::new(),
            visible: Vec::new(),
            visible_dirty: true,
            view_generation: 1,
            error: None,
        };
        view.install_roots();
        view.refresh_visible();
        view
    }
}

impl TreeView {
    fn install_roots(&mut self) {
        let roots = ROOTS
            .iter()
            .map(|(id, name)| {
                let id = RowId(*id);
                self.records.insert(
                    id,
                    Record {
                        name: (*name).into(),
                        kind: "仓库",
                        code: format!("WH-{id:02}", id = id.0),
                        status: "可用",
                    },
                );
                TreeNode {
                    id,
                    has_children: true,
                }
            })
            .collect();
        if let Err(error) = self.model.set_roots(roots) {
            self.error = Some(error.to_string());
        }
        self.visible_dirty = true;
    }

    fn fixture_children(parent: RowId) -> (Vec<TreeNode>, Vec<(RowId, Record)>) {
        let is_root = parent.0 <= 4;
        let count = if is_root { 3 } else { 8 };
        let mut nodes = Vec::with_capacity(count);
        let mut records = Vec::with_capacity(count);
        for index in 1..=count {
            let id = RowId(parent.0 * 100 + index as u128);
            let has_children = is_root;
            nodes.push(TreeNode { id, has_children });
            records.push((
                id,
                Record {
                    name: if is_root {
                        format!("{}号库区", index)
                    } else {
                        format!("{}-{:02}货位", parent.0, index)
                    },
                    kind: if is_root { "库区" } else { "货位" },
                    code: if is_root {
                        format!("AREA-{}-{index:02}", parent.0)
                    } else {
                        format!("BIN-{}-{index:02}", parent.0)
                    },
                    status: if index % 5 == 0 {
                        "盘点中"
                    } else {
                        "可用"
                    },
                },
            ));
        }
        (nodes, records)
    }

    fn schedule(&mut self, ticket: LoadTicket, now: f64) {
        let parent = ticket.parent;
        let should_fail = parent == RowId(2) && self.failed_once.insert(parent);
        let (children, records) = Self::fixture_children(parent);
        self.pending.push(PendingLoad {
            ticket,
            due: now + LOAD_DELAY_SECONDS,
            result: if should_fail {
                Err("本地夹具首次加载失败，点击箭头重试".into())
            } else {
                Ok(children)
            },
            records,
        });
        self.visible_dirty = true;
    }

    fn poll(&mut self, now: f64) {
        let mut index = 0;
        while index < self.pending.len() {
            if self.pending[index].due > now {
                index += 1;
                continue;
            }
            let pending = self.pending.remove(index);
            let succeeded = pending.result.is_ok();
            let before: Vec<_> = self.model.visible().iter().map(|row| row.id).collect();
            match self.model.complete(pending.ticket, pending.result) {
                Ok(true) => {
                    if succeeded {
                        self.records.extend(pending.records);
                    }
                    self.visible_dirty = true;
                    let after: Vec<_> = self.model.visible().iter().map(|row| row.id).collect();
                    if before != after {
                        self.rebind_visible();
                    }
                }
                Ok(false) => {}
                Err(error) => {
                    self.error = Some(error.to_string());
                    self.visible_dirty = true;
                }
            }
        }
    }

    fn refresh(&mut self) {
        let generation = self.model.generation().wrapping_add(1);
        self.model.reset(generation);
        self.records.clear();
        self.pending.clear();
        self.state.row_selection.clear();
        self.error = None;
        self.install_roots();
        self.rebind_visible();
    }

    fn refresh_visible(&mut self) {
        if !self.visible_dirty {
            return;
        }
        self.visible = self.model.visible();
        self.state.tree_rows = self
            .visible
            .iter()
            .map(|row| {
                (
                    row.id,
                    TreeRowVisual {
                        depth: row.depth,
                        expanded: row.expanded,
                        has_children: row.has_children,
                        loading: row.loading,
                        error: row.error.clone(),
                    },
                )
            })
            .collect();
        self.visible_dirty = false;
    }

    fn rebind_visible(&mut self) {
        self.view_generation = self.view_generation.wrapping_add(1);
        self.state.rebind_page(self.view_generation, false);
        self.visible_dirty = true;
        self.refresh_visible();
        if self
            .state
            .row_selection
            .active
            .is_some_and(|id| !self.visible.iter().any(|row| row.id == id))
        {
            self.state.row_selection.active = None;
        }
    }

    fn toggle(&mut self, id: RowId, now: f64) {
        let before: Vec<_> = self.visible.iter().map(|row| row.id).collect();
        match self.model.toggle(id) {
            Ok(ticket) => {
                self.visible_dirty = true;
                self.refresh_visible();
                let after: Vec<_> = self.visible.iter().map(|row| row.id).collect();
                if before != after {
                    self.rebind_visible();
                }
                if let Some(ticket) = ticket {
                    self.schedule(ticket, now);
                    self.refresh_visible();
                }
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let now = ui.input(|input| input.time);
        self.poll(now);
        self.refresh_visible();

        ui.horizontal(|ui| {
            if ui
                .button("刷新")
                .on_hover_text("本地 150 ms 延迟夹具；不代表服务端响应")
                .clicked()
            {
                self.refresh();
            }
            if !self.pending.is_empty() {
                ui.spinner()
                    .on_hover_text("本地 150 ms 延迟夹具；不代表服务端响应");
            }
            if let Some(error) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
        });

        let visible = &self.visible;
        let records = &self.records;
        let available = ui.available_rect_before_wrap();
        let footer_top = (available.bottom() - 26.0).max(available.top());
        let output = ui
            .scope_builder(
                egui::UiBuilder::new().max_rect(egui::Rect::from_min_max(
                    available.min,
                    egui::pos2(available.right(), footer_top),
                )),
                |ui| {
                    DataGrid {
                        id: ui.id().with("warehouse-tree"),
                        columns: &mut self.columns,
                        rows: visible.len(),
                        state: &mut self.state,
                        row_id: Some(&|index| visible[index].id),
                    }
                    .show(ui, |row, column| {
                        let Some(record) = visible.get(row).and_then(|row| records.get(&row.id))
                        else {
                            return String::new();
                        };
                        match column.0 {
                            0 => record.name.clone(),
                            1 => record.kind.into(),
                            2 => record.code.clone(),
                            3 => record.status.into(),
                            _ => String::new(),
                        }
                    })
                },
            )
            .inner;
        if let Some(id) = output.toggle_tree {
            self.toggle(id, now);
        }
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(egui::Rect::from_min_max(
                egui::pos2(available.left(), footer_top),
                available.max,
            )),
            |ui| {
                ui.label(format!(
                    "已选 {} 项",
                    self.state.row_selection.checked().len()
                ));
            },
        );

        if let Some(next) = self
            .pending
            .iter()
            .map(|pending| pending.due)
            .min_by(f64::total_cmp)
        {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64((next - now).max(0.001)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_footer_stays_inside_host_after_loading_children() {
        let mut view = TreeView::default();
        view.toggle(RowId(1), 0.0);
        view.poll(LOAD_DELAY_SECONDS);
        view.refresh_visible();
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 500.0),
                )),
                ..Default::default()
            },
            |root| {
                egui::CentralPanel::default().show(root, |ui| view.ui(ui));
            },
        );
        let footer = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.text().starts_with("已选 ") => {
                    Some(text.visual_bounding_rect())
                }
                _ => None,
            })
            .expect("footer text is painted");
        assert!(
            footer.top() > 450.0 && footer.bottom() <= 500.0,
            "{footer:?}"
        );
        output.textures_delta.clear();
    }
    #[test]
    fn loads_only_the_requested_parent() {
        let mut view = TreeView::default();
        view.toggle(RowId(1), 0.0);
        view.poll(LOAD_DELAY_SECONDS);
        view.refresh_visible();
        assert_eq!(view.visible.len(), 7);
        assert!(view.visible.iter().any(|row| row.id == RowId(101)));
        assert!(!view.visible.iter().any(|row| row.id == RowId(201)));
    }

    #[test]
    fn refresh_discards_a_late_load() {
        let mut view = TreeView::default();
        view.toggle(RowId(1), 0.0);
        let late = view.pending.remove(0);
        view.refresh();
        assert_eq!(view.model.complete(late.ticket, late.result), Ok(false));
        view.refresh_visible();
        assert_eq!(view.visible.len(), ROOTS.len());
        assert!(!view.records.contains_key(&RowId(101)));
    }
}
