use crate::GridState;
use egui::{Id, Rect, Response, Sense, Ui, UiBuilder};
use erp_grid_core::{Column, ColumnId, RowId};
use std::ops::Range;

const MAX_ACCESSIBLE_VALUE_BYTES: usize = 4096;

fn bounded_value(value: String) -> String {
    if value.len() <= MAX_ACCESSIBLE_VALUE_BYTES {
        return value;
    }
    let mut end = MAX_ACCESSIBLE_VALUE_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

fn set_bounds(node: &mut accesskit::Node, rect: Rect) {
    node.set_bounds(accesskit::Rect {
        x0: rect.min.x.into(),
        y0: rect.min.y.into(),
        x1: rect.max.x.into(),
        y1: rect.max.y.into(),
    });
}

pub(crate) struct VisibleGrid<'a> {
    pub id: Id,
    pub response: &'a Response,
    pub bounds: Rect,
    pub rows: usize,
    pub row_number_start: u64,
    pub columns: &'a [Column],
    pub visible_rows: Range<usize>,
    pub visible_columns: &'a [usize],
    pub state: &'a GridState,
    pub row_id: Option<&'a dyn Fn(usize) -> RowId>,
}

/// Adds semantics only for cells already admitted by viewport virtualization.
pub(crate) fn expose(
    ui: &mut Ui,
    grid: VisibleGrid<'_>,
    cell_text: &impl Fn(usize, ColumnId) -> String,
    cell_rect: impl Fn(usize, usize) -> Rect,
    header_rect: impl Fn(usize) -> Rect,
) {
    let VisibleGrid {
        id,
        response,
        bounds,
        rows,
        row_number_start,
        columns,
        visible_rows,
        visible_columns,
        state,
        row_id,
    } = grid;

    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Data grid"));
    let Some(()) = ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(accesskit::Role::Grid);
        node.set_label("Data grid");
        node.set_row_count(rows);
        node.set_column_count(columns.len());
        node.set_multiselectable();
        if state.read_only {
            node.set_read_only();
        }
        set_bounds(node, bounds);
    }) else {
        return;
    };

    let semantic_ui = ui.new_child(
        UiBuilder::new()
            .id_salt(id.with("accessibility"))
            .max_rect(bounds)
            .accessibility_parent(response.id),
    );
    semantic_ui
        .ctx()
        .accesskit_node_builder(semantic_ui.unique_id(), |node| {
            node.set_role(accesskit::Role::Group);
            node.set_label("Visible data grid cells");
            set_bounds(node, bounds);
        });

    for &column_index in visible_columns {
        let column = &columns[column_index];
        let rect = header_rect(column_index);
        let header_id = id.with(("accessibility-header", column.id.0));
        let _ = semantic_ui.interact(rect, header_id, Sense::hover());
        semantic_ui.ctx().accesskit_node_builder(header_id, |node| {
            node.set_role(accesskit::Role::ColumnHeader);
            node.set_label(column.title.clone());
            node.set_column_index(column_index);
            set_bounds(node, rect);
        });
    }

    let mut active_descendant = None;
    for row in visible_rows {
        let stable_row = row_id.map(|map| map(row));
        let semantic_row = stable_row.map_or((false, row as u128), |row| (true, row.0));
        let checked = stable_row.is_some_and(|row| state.row_selection.checked().contains(&row));
        for &column_index in visible_columns {
            let column = &columns[column_index];
            let cell = erp_grid_core::Cell {
                row,
                column: column_index,
            };
            let rect = cell_rect(row, column_index);
            let cell_id = id.with(("accessibility-cell", semantic_row, column.id.0));
            let _ = semantic_ui.interact(rect, cell_id, Sense::hover());
            let value = bounded_value(cell_text(row, column.id));
            let row_number = row_number_start
                .saturating_add(row as u64)
                .saturating_add(1);
            semantic_ui.ctx().accesskit_node_builder(cell_id, |node| {
                node.set_role(accesskit::Role::GridCell);
                node.set_label(format!("Row {row_number}, column {}", column.title));
                node.set_value(value);
                node.set_row_index(row);
                node.set_column_index(column_index);
                if state.selection.contains(cell) {
                    node.set_selected(true);
                }
                if checked {
                    node.set_state_description("Row checked");
                }
                if state.read_only || matches!(&column.edit_kind, erp_grid_core::EditKind::ReadOnly)
                {
                    node.set_read_only();
                }
                set_bounds(node, rect);
            });
            if state.focus == Some(cell) {
                active_descendant = Some(cell_id.accesskit_id());
            }
        }
    }

    if let Some(active_descendant) = active_descendant {
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_active_descendant(active_descendant);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DataGrid;
    use egui::{Pos2, RawInput, Vec2};
    use erp_grid_core::Cell;

    #[test]
    fn accesskit_tree_is_bounded_and_tracks_the_active_stable_cell() {
        let context = egui::Context::default();
        context.enable_accesskit();
        let grid_id = Id::new("accesskit-grid-test");
        let mut columns = vec![
            Column::new(ColumnId(7), "SKU", 120.0),
            Column::new(ColumnId(9), "Name", 160.0),
        ];
        let mut state = GridState {
            quick_search_open: false,
            focus: Some(Cell { row: 1, column: 0 }),
            ..Default::default()
        };
        state.visual_ids = columns.iter().map(|column| column.id).collect();
        let stable_row = |row| RowId(10_000 + row as u128);

        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(420.0, 240.0))),
                ..Default::default()
            },
            |root| {
                egui::CentralPanel::default().show(root, |ui| {
                    DataGrid {
                        id: grid_id,
                        columns: &mut columns,
                        rows: 1_000_000,
                        state: &mut state,
                        row_id: Some(&stable_row),
                    }
                    .show(ui, |row, column| format!("{row}:{}", column.0));
                });
            },
        );

        let update = output
            .platform_output
            .accesskit_update
            .as_ref()
            .expect("enabling AccessKit must produce a tree update");
        let grid = update
            .nodes
            .iter()
            .find_map(|(_, node)| (node.role() == accesskit::Role::Grid).then_some(node))
            .expect("the DataGrid role must be exposed");
        let header_count = update
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == accesskit::Role::ColumnHeader)
            .count();
        let cell_count = update
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == accesskit::Role::GridCell)
            .count();
        let expected_active = grid_id
            .with(("accessibility-cell", (true, 10_001_u128), 7_u64))
            .accesskit_id();

        assert_eq!(header_count, 2);
        assert!(cell_count > 0);
        assert!(cell_count <= 32, "only viewport cells may enter the tree");
        assert_eq!(grid.active_descendant(), Some(expected_active));
        output.textures_delta.clear();
    }
}
