use egui::{Id, Ui};
use erp_grid_core::{Column, ColumnId, move_column};

#[derive(Clone, Copy)]
pub(crate) enum ColumnAction {
    Visibility(ColumnId, bool),
    Width(ColumnId, f32),
    Pin(ColumnId, bool),
    PinRight(ColumnId, bool),
    Move(ColumnId, Option<ColumnId>),
}

pub(crate) fn apply(columns: &mut Vec<Column>, action: ColumnAction) -> bool {
    let changed = match action {
        ColumnAction::Visibility(id, visible) => {
            let count = columns.iter().filter(|c| c.visible).count();
            if !visible && count <= 1 {
                return false;
            }
            if let Some(column) = columns.iter_mut().find(|c| c.id == id) {
                let changed = column.visible != visible;
                column.visible = visible;
                changed
            } else {
                false
            }
        }
        ColumnAction::Pin(id, pinned) => {
            if let Some(column) = columns.iter_mut().find(|c| c.id == id) {
                let changed = column.pinned != pinned || column.pinned_right;
                column.pinned = pinned;
                column.pinned_right = false;
                changed
            } else {
                false
            }
        }
        ColumnAction::PinRight(id, pinned) => {
            if let Some(column) = columns.iter_mut().find(|c| c.id == id) {
                let changed = column.pinned_right != pinned || column.pinned;
                column.pinned = false;
                column.pinned_right = pinned;
                changed
            } else {
                false
            }
        }
        ColumnAction::Width(id, width) => {
            if let Some(column) = columns.iter_mut().find(|c| c.id == id) {
                let before = column.width();
                column.resize(width);
                before != column.width()
            } else {
                false
            }
        }
        ColumnAction::Move(id, before) => move_column(columns, id, before),
    };
    columns.sort_by_key(Column::pin_rank);
    changed
}

pub(crate) fn show(
    ui: &mut Ui,
    id: Id,
    open: &mut bool,
    query: &mut String,
    columns: &mut Vec<Column>,
) -> bool {
    let mut action = None;
    egui::Window::new("Columns")
        .id(id.with("column-config"))
        .open(open)
        .default_width(440.0)
        .resizable(true)
        .show(ui.ctx(), |ui| {
            ui.add(egui::TextEdit::singleline(query).hint_text("Find column"));
            let needle = query.to_lowercase();
            let visible_count = columns.iter().filter(|c| c.visible).count();
            egui::ScrollArea::vertical()
                .max_height(440.0)
                .show(ui, |ui| {
                    for (index, column) in columns.iter().enumerate() {
                        if !column.title.to_lowercase().contains(&needle) {
                            continue;
                        }
                        ui.push_id(column.id.0, |ui| {
                            ui.horizontal(|ui| {
                                let mut visible = column.visible;
                                if ui
                                    .add_enabled(
                                        !visible || visible_count > 1,
                                        egui::Checkbox::without_text(&mut visible),
                                    )
                                    .on_hover_text("Show / hide column")
                                    .changed()
                                {
                                    action = Some(ColumnAction::Visibility(column.id, visible));
                                }
                                ui.add_sized(
                                    [130.0, 22.0],
                                    egui::Label::new(&column.title).truncate(),
                                );
                                let mut width = column.width();
                                let (min_width, max_width) = column.width_bounds();
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut width)
                                            .range(min_width..=max_width)
                                            .speed(1.0)
                                            .suffix(" px"),
                                    )
                                    .on_hover_text(
                                        format!(
                                            "Column width ({min_width:.0}–{max_width:.0} px); drag the number or double-click to type"
                                        ),
                                    )
                                    .changed()
                                {
                                    action = Some(ColumnAction::Width(column.id, width));
                                }
                                let mut pinned = column.pinned;
                                if ui
                                    .checkbox(&mut pinned, "Lock")
                                    .on_hover_text("Keep this column on the left while scrolling")
                                    .changed()
                                {
                                    action = Some(ColumnAction::Pin(column.id, pinned));
                                }
                                let mut right = column.pin_rank() == 2;
                                if ui.checkbox(&mut right, "Right").on_hover_text("Keep this column on the right while scrolling").changed() {
                                    action = Some(ColumnAction::PinRight(column.id, right));
                                }
                                let previous = index
                                    .checked_sub(1)
                                    .and_then(|i| columns.get(i))
                                    .filter(|c| c.pin_rank() == column.pin_rank());
                                if ui
                                    .add_enabled(previous.is_some(), egui::Button::new("↑"))
                                    .on_hover_text("Move left within this group")
                                    .clicked()
                                {
                                    action =
                                        Some(ColumnAction::Move(column.id, previous.map(|c| c.id)));
                                }
                                let next =
                                    columns.get(index + 1).filter(|c| c.pin_rank() == column.pin_rank());
                                if ui
                                    .add_enabled(next.is_some(), egui::Button::new("↓"))
                                    .on_hover_text("Move right within this group")
                                    .clicked()
                                {
                                    action = Some(ColumnAction::Move(
                                        column.id,
                                        columns.get(index + 2).map(|c| c.id),
                                    ));
                                }
                            });
                        });
                    }
                });
        });
    action.is_some_and(|action| apply(columns, action))
}

/// Preserve a usable center viewport; release rightmost locks first on resize.
pub(crate) fn fit_pinned(columns: &mut [Column], budget: f32) -> bool {
    let mut total: f32 = columns
        .iter()
        .filter(|column| column.visible && column.is_pinned())
        .map(Column::width)
        .sum();
    let mut changed = false;
    for column in columns
        .iter_mut()
        .rev()
        .filter(|column| column.visible && column.is_pinned())
    {
        if total <= budget.max(0.0) {
            break;
        }
        total -= column.width();
        column.pinned = false;
        column.pinned_right = false;
        changed = true;
    }
    if changed {
        columns.sort_by_key(Column::pin_rank);
    }
    changed
}
