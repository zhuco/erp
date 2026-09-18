use super::*;
use column_config::{ColumnAction, apply, fit_pinned};

#[test]
fn grouped_header_adds_one_row_and_does_not_turn_group_click_into_sort() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    let mut columns = vec![
        Column::new(ColumnId(1), "A", 120.0),
        Column::new(ColumnId(2), "B", 120.0),
    ];
    let before = frame(&ctx, &mut state, &mut columns, vec![]);
    let normal_y = text_position(&before, "1:0").y;
    state.header_groups = vec![erp_grid_core::HeaderGroup::new(
        "Amounts",
        [ColumnId(1), ColumnId(2)],
    )];
    let grouped = frame(&ctx, &mut state, &mut columns, vec![]);
    assert_eq!(text_position(&grouped, "1:0").y - normal_y, 28.0);
    let group = text_position(&grouped, "Amounts") + Vec2::new(5.0, 5.0);
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(group)],
    );
    for pressed in [true, false] {
        frame(
            &ctx,
            &mut state,
            &mut columns,
            vec![egui::Event::PointerButton {
                pos: group,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            }],
        );
    }
    assert!(state.sorts.is_empty());
    assert!(state.focus.is_none());
}

#[test]
fn right_pinned_column_keeps_screen_position_and_uses_its_own_cell_identity() {
    let ctx = egui::Context::default();
    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        ctx.style_mut_of(theme, |style| style.animation_time = 0.0);
    }
    let mut state = GridState::default();
    let mut columns: Vec<_> = (1..=12)
        .map(|id| Column::new(ColumnId(id), format!("C{id}"), 120.0))
        .collect();
    assert!(apply(&mut columns, ColumnAction::Pin(ColumnId(1), true)));
    assert!(apply(
        &mut columns,
        ColumnAction::PinRight(ColumnId(12), true)
    ));
    frame(&ctx, &mut state, &mut columns, vec![]);
    let before = frame(&ctx, &mut state, &mut columns, vec![]);
    let right = text_position(&before, "12:0");
    let left = text_position(&before, "1:0");
    state.scroll.x = 240.0;
    let after = frame(&ctx, &mut state, &mut columns, vec![]);
    assert_eq!(text_position(&after, "12:0"), right);
    assert_eq!(text_position(&after, "1:0"), left);
    let pos = right + Vec2::new(8.0, 6.0);
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(pos)],
    );
    for pressed in [true, false] {
        frame(
            &ctx,
            &mut state,
            &mut columns,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            }],
        );
    }
    assert_eq!(
        state.focus.map(|cell| columns
            .iter()
            .filter(|c| c.visible)
            .nth(cell.column)
            .unwrap()
            .id),
        Some(ColumnId(12))
    );
}

#[test]
fn right_pin_changes_are_exclusive_and_share_the_frozen_width_budget() {
    let mut columns = vec![
        Column::new(ColumnId(1), "A", 120.0),
        Column::new(ColumnId(2), "B", 120.0),
    ];
    apply(&mut columns, ColumnAction::Pin(ColumnId(1), true));
    apply(&mut columns, ColumnAction::PinRight(ColumnId(1), true));
    let right = columns.iter().find(|c| c.id == ColumnId(1)).unwrap();
    assert!(!right.pinned && right.pinned_right);
    apply(&mut columns, ColumnAction::Pin(ColumnId(2), true));
    assert!(fit_pinned(&mut columns, 120.0));
    assert!(
        columns
            .iter()
            .filter(|c| c.is_pinned())
            .map(Column::width)
            .sum::<f32>()
            <= 120.0
    );
}

fn frame(
    ctx: &egui::Context,
    state: &mut GridState,
    columns: &mut Vec<Column>,
    events: Vec<egui::Event>,
) -> Vec<(String, Pos2)> {
    frame_rows(ctx, state, columns, events, 100)
}
fn frame_rows(
    ctx: &egui::Context,
    state: &mut GridState,
    columns: &mut Vec<Column>,
    events: Vec<egui::Event>,
    rows: usize,
) -> Vec<(String, Pos2)> {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            events,
            ..Default::default()
        },
        |root| {
            egui::CentralPanel::default().show(root, |ui| {
                DataGrid {
                    id: Id::new("column-test"),
                    state,
                    columns,
                    rows,
                    row_id: Some(&|row| RowId(row as u128)),
                }
                .show(ui, |row, column| format!("{}:{row}", column.0));
            });
        },
    );
    fn collect(shape: &egui::epaint::Shape, text: &mut Vec<(String, Pos2)>) {
        match shape {
            egui::epaint::Shape::Text(value) => text.push((value.galley.text().into(), value.pos)),
            egui::epaint::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, text);
                }
            }
            _ => {}
        }
    }
    let mut text = Vec::new();
    for shape in &output.shapes {
        collect(&shape.shape, &mut text);
    }
    output.textures_delta.clear();
    text
}
fn click(
    ctx: &egui::Context,
    state: &mut GridState,
    columns: &mut Vec<Column>,
    pos: Pos2,
    button: egui::PointerButton,
) -> Vec<(String, Pos2)> {
    frame(ctx, state, columns, vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        frame(
            ctx,
            state,
            columns,
            vec![egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: Default::default(),
            }],
        );
    }
    frame(ctx, state, columns, vec![])
}
fn text_position(text: &[(String, Pos2)], label: &str) -> Pos2 {
    text.iter()
        .find(|(text, _)| text == label)
        .unwrap_or_else(|| panic!("Missing {label}; {text:?}"))
        .1
}

#[test]
fn header_menu_opens_settings_and_configuration_changes_real_columns() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    let mut columns = vec![
        Column::new(ColumnId(1), "SKU", 120.0),
        Column::new(ColumnId(2), "Name", 120.0),
    ];
    frame(&ctx, &mut state, &mut columns, vec![]);
    let text = click(
        &ctx,
        &mut state,
        &mut columns,
        Pos2::new(140.0, 22.0),
        egui::PointerButton::Secondary,
    );
    let pos = text_position(&text, "Lock to left") + Vec2::splat(4.0);
    click(
        &ctx,
        &mut state,
        &mut columns,
        pos,
        egui::PointerButton::Primary,
    );
    assert!(columns[0].pinned);
    let text = click(
        &ctx,
        &mut state,
        &mut columns,
        Pos2::new(140.0, 22.0),
        egui::PointerButton::Secondary,
    );
    let pos = text_position(&text, "Columns...") + Vec2::splat(4.0);
    let text = click(
        &ctx,
        &mut state,
        &mut columns,
        pos,
        egui::PointerButton::Primary,
    );
    assert!(state.column_config_open);
    let pos = text_position(&text, "Lock") + Vec2::splat(4.0);
    click(
        &ctx,
        &mut state,
        &mut columns,
        pos,
        egui::PointerButton::Primary,
    );
    assert!(
        !columns[0].pinned,
        "panel checkbox must change the column model"
    );
    state.column_config_open = false;
    for _ in 0..30 {
        frame(&ctx, &mut state, &mut columns, vec![]);
    }
    let text = click(
        &ctx,
        &mut state,
        &mut columns,
        Pos2::new(140.0, 22.0),
        egui::PointerButton::Secondary,
    );
    let pos = text_position(&text, "Hide column") + Vec2::splat(4.0);
    click(
        &ctx,
        &mut state,
        &mut columns,
        pos,
        egui::PointerButton::Primary,
    );
    assert!(!columns[0].visible);
    assert!(columns[1].visible);
}

#[test]
fn frozen_columns_keep_coordinates_and_hit_identity_while_hidden_columns_are_skipped() {
    let ctx = egui::Context::default();
    let mut state = GridState {
        quick_search_open: false,
        ..Default::default()
    };
    let mut columns: Vec<_> = (1..=12)
        .map(|id| Column::new(ColumnId(id), format!("C{id}"), 120.0))
        .collect();
    columns[0].pinned = true;
    columns[1].visible = false;
    let initial = frame(&ctx, &mut state, &mut columns, vec![]);
    state.scroll.x = 240.0;
    let scrolled = frame(&ctx, &mut state, &mut columns, vec![]);
    assert_eq!(
        text_position(&initial, "C1").x,
        text_position(&scrolled, "C1").x
    );
    assert!(
        !scrolled
            .iter()
            .any(|(text, _)| text == "C2" || text.starts_with("2:"))
    );
    click(
        &ctx,
        &mut state,
        &mut columns,
        Pos2::new(140.0, 53.0),
        egui::PointerButton::Primary,
    );
    assert_eq!(state.focus, Some(Cell { row: 0, column: 0 }));
    click(
        &ctx,
        &mut state,
        &mut columns,
        Pos2::new(260.0, 53.0),
        egui::PointerButton::Primary,
    );
    let visible: Vec<_> = columns.iter().filter(|c| c.visible).cloned().collect();
    assert_eq!(visible[state.focus.unwrap().column].id, ColumnId(5));
    assert_eq!(
        selection_tsv(&state.selection, &visible, 100, |_, id| id.0.to_string()).unwrap(),
        "5"
    );
}

#[test]
fn column_layout_preserves_hidden_identity_groups_and_one_visible_column() {
    let mut columns: Vec<_> = (1..=4)
        .map(|id| Column::new(ColumnId(id), format!("C{id}"), 120.0))
        .collect();
    apply(&mut columns, ColumnAction::Pin(ColumnId(3), true));
    apply(
        &mut columns,
        ColumnAction::Move(ColumnId(4), Some(ColumnId(1))),
    );
    assert_eq!(
        columns.iter().map(|c| c.id.0).collect::<Vec<_>>(),
        [3, 4, 1, 2]
    );
    for id in [1, 2, 4] {
        assert!(apply(
            &mut columns,
            ColumnAction::Visibility(ColumnId(id), false)
        ));
    }
    assert!(!apply(
        &mut columns,
        ColumnAction::Visibility(ColumnId(3), false)
    ));
    assert!(apply(
        &mut columns,
        ColumnAction::Visibility(ColumnId(1), true)
    ));
    assert!(apply(&mut columns, ColumnAction::Pin(ColumnId(1), true)));
    assert!(fit_pinned(&mut columns, 150.0));
    assert!(columns[0].pinned);
    assert_eq!(columns.iter().filter(|c| c.pinned && c.visible).count(), 1);
}

#[test]
fn frozen_and_scrolling_filter_inputs_keep_column_ids() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    let mut columns: Vec<_> = (1..=12)
        .map(|id| Column::new(ColumnId(id), format!("C{id}"), 120.0))
        .collect();
    columns[0].pinned = true;
    columns[1].visible = false;
    frame(&ctx, &mut state, &mut columns, vec![]);
    state.scroll.x = 240.0;
    frame(&ctx, &mut state, &mut columns, vec![]);
    click(
        &ctx,
        &mut state,
        &mut columns,
        Pos2::new(140.0, 52.0),
        egui::PointerButton::Primary,
    );
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::Text("frozen".into())],
    );
    click(
        &ctx,
        &mut state,
        &mut columns,
        Pos2::new(260.0, 52.0),
        egui::PointerButton::Primary,
    );
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::Text("scrolling".into())],
    );
    assert_eq!(
        state.quick_search.get(&ColumnId(1)),
        Some(&erp_grid_core::FilterDraft::Text("frozen".into()))
    );
    assert_eq!(
        state.quick_search.get(&ColumnId(5)),
        Some(&erp_grid_core::FilterDraft::Text("scrolling".into()))
    );
    assert!(state.selection.regions().is_empty());
    assert!(state.row_selection.checked().is_empty());
}

#[test]
fn short_table_horizontal_scrollbar_remains_at_control_bottom() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    let mut columns: Vec<_> = (1..=12)
        .map(|id| Column::new(ColumnId(id), format!("C{id}"), 120.0))
        .collect();
    for _ in 0..3 {
        frame_rows(&ctx, &mut state, &mut columns, vec![], 10);
    }
    let pos = Pos2::new(650.0, 587.0);
    frame_rows(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(pos)],
        10,
    );
    for pressed in [true, false] {
        frame_rows(
            &ctx,
            &mut state,
            &mut columns,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            }],
            10,
        );
    }
    assert!(
        state.scroll.x > 0.0,
        "bottom scrollbar must remain clickable below ten rows: {:?}",
        state.scroll
    );
    assert_eq!(state.scroll.y, 0.0);
    assert!(state.selection.regions().is_empty());
}

#[test]
fn structural_columns_stay_fixed_and_are_outside_horizontal_scrollbar() {
    let ctx = egui::Context::default();
    let mut state = GridState {
        row_number_start: 100,
        ..Default::default()
    };
    let mut columns: Vec<_> = (1..=12)
        .map(|id| Column::new(ColumnId(id), format!("C{id}"), 120.0))
        .collect();
    let initial = frame_rows(&ctx, &mut state, &mut columns, vec![], 10);
    let serial = text_position(&initial, "101");
    assert!(serial.x < 104.0);
    assert!(text_position(&initial, "序").x < 104.0);
    let pos = Pos2::new(70.0, 587.0);
    frame_rows(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(pos)],
        10,
    );
    for pressed in [true, false] {
        frame_rows(
            &ctx,
            &mut state,
            &mut columns,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            }],
            10,
        );
    }
    assert_eq!(
        state.scroll.x, 0.0,
        "structural area must not activate the horizontal scrollbar"
    );
    state.scroll.x = 240.0;
    let scrolled = frame_rows(&ctx, &mut state, &mut columns, vec![], 10);
    assert_eq!(text_position(&scrolled, "101"), serial);
    let pos = Pos2::new(22.0, 80.0);
    frame_rows(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(pos)],
        10,
    );
    for pressed in [true, false] {
        frame_rows(
            &ctx,
            &mut state,
            &mut columns,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            }],
            10,
        );
    }
    assert_eq!(state.row_selection.targets(), [RowId(0)]);
    assert!(columns.iter().all(|column| column.title.starts_with('C')));
}
