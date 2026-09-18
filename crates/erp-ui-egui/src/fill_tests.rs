use super::*;

fn button(pos: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    }
}

fn frame(
    ctx: &egui::Context,
    state: &mut GridState,
    columns: &mut Vec<Column>,
    events: Vec<egui::Event>,
) -> GridOutput {
    let mut grid_output = None;
    let mut frame = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            events,
            ..Default::default()
        },
        |root| {
            egui::CentralPanel::default().show(root, |ui| {
                grid_output = Some(
                    DataGrid {
                        id: Id::new("fill-regression"),
                        columns,
                        state,
                        rows: 20,
                        row_id: Some(&|row| RowId(1_000 + row as u128)),
                    }
                    .show(ui, |row, column| format!("{row}:{}", column.0)),
                );
            });
        },
    );
    frame.textures_delta.clear();
    grid_output.expect("grid output")
}

fn editable_columns() -> Vec<Column> {
    let mut columns = vec![
        Column::new(ColumnId(1), "A", 120.0),
        Column::new(ColumnId(2), "B", 120.0),
    ];
    for column in &mut columns {
        column.edit_kind = erp_grid_core::EditKind::Text;
    }
    columns
}

#[test]
fn fill_handle_emits_horizontal_target_without_changing_source() {
    let ctx = egui::Context::default();
    let mut state = GridState {
        fill_enabled: true,
        ..Default::default()
    };
    let mut columns = editable_columns();
    frame(&ctx, &mut state, &mut columns, vec![]);
    let source = CellRect {
        top: 1,
        bottom: 2,
        left: 0,
        right: 1,
    };
    state
        .selection
        .apply(source, SelectionOperation::Replace)
        .unwrap();
    frame(&ctx, &mut state, &mut columns, vec![]);
    let handle = Pos2::new(CHECK_WIDTH + NUMBER_WIDTH + 8.0 + 120.0, 124.0);
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(handle), button(handle, true)],
    );
    let target = Pos2::new(300.0, 110.0);
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(target)],
    );
    let output = frame(&ctx, &mut state, &mut columns, vec![button(target, false)]);
    assert_eq!(
        output.fill,
        Some(FillIntent {
            source,
            target: CellRect {
                top: 1,
                bottom: 2,
                left: 1,
                right: 2
            }
        })
    );
    assert_eq!(state.selection.regions(), &[source]);
}

#[test]
fn fill_handle_emits_only_new_rows_once_and_preserves_selection() {
    let ctx = egui::Context::default();
    let mut state = GridState {
        fill_enabled: true,
        ..Default::default()
    };
    let source = CellRect {
        top: 1,
        left: 0,
        bottom: 2,
        right: 1,
    };
    let mut columns = editable_columns();
    frame(&ctx, &mut state, &mut columns, vec![]);
    state
        .selection
        .apply(source, SelectionOperation::Replace)
        .expect("valid selection");
    frame(&ctx, &mut state, &mut columns, vec![]);

    // One 120px business column starts after the 96px structure rail and 8px panel inset.
    let handle = Pos2::new(CHECK_WIDTH + NUMBER_WIDTH + 8.0 + 120.0, 124.0);
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(handle), button(handle, true)],
    );
    let target = Pos2::new(CHECK_WIDTH + NUMBER_WIDTH + 8.0 + 120.0, 194.0);
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(target)],
    );
    let output = frame(&ctx, &mut state, &mut columns, vec![button(target, false)]);
    assert_eq!(
        output.fill,
        Some(FillIntent {
            source,
            target: CellRect {
                top: 2,
                left: 0,
                bottom: 5,
                right: 1,
            },
        })
    );
    assert_eq!(state.selection.regions(), &[source]);
    assert!(!output.columns_changed);
    assert!(!output.sorts_changed);
    assert!(frame(&ctx, &mut state, &mut columns, vec![]).fill.is_none());
}

#[test]
fn fill_requires_opt_in_editability_and_loaded_release() {
    assert!(!GridState::default().fill_enabled);
    assert_eq!(
        fill_target(
            CellRect {
                top: 3,
                left: 0,
                bottom: 5,
                right: 1,
            },
            1,
            20,
        ),
        Some(CellRect {
            top: 1,
            left: 0,
            bottom: 3,
            right: 1,
        })
    );
    assert!(
        fill_target(
            CellRect::between(Cell { row: 0, column: 0 }, Cell { row: 0, column: 0 }),
            20,
            20,
        )
        .is_none()
    );

    let ctx = egui::Context::default();
    let source = CellRect::between(Cell { row: 1, column: 0 }, Cell { row: 1, column: 0 });
    let mut state = GridState {
        fill_enabled: true,
        ..Default::default()
    };
    let mut columns = editable_columns();
    frame(&ctx, &mut state, &mut columns, vec![]);
    state
        .selection
        .apply(source, SelectionOperation::Replace)
        .expect("valid selection");
    frame(&ctx, &mut state, &mut columns, vec![]);
    let handle = Pos2::new(224.0, 124.0);
    let outside_grid = Pos2::new(50.0, 194.0);
    frame(&ctx, &mut state, &mut columns, vec![button(handle, true)]);
    assert!(
        frame(
            &ctx,
            &mut state,
            &mut columns,
            vec![button(outside_grid, false)],
        )
        .fill
        .is_none()
    );

    state
        .selection
        .apply(source, SelectionOperation::Replace)
        .expect("valid selection");
    columns[0].edit_kind = erp_grid_core::EditKind::ReadOnly;
    frame(&ctx, &mut state, &mut columns, vec![button(handle, true)]);
    assert!(
        frame(
            &ctx,
            &mut state,
            &mut columns,
            vec![button(Pos2::new(180.0, 194.0), false)],
        )
        .fill
        .is_none()
    );
}
