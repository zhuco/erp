use super::*;

#[test]
fn data_area_modifiers_check_rows_without_touching_copy_rectangles() {
    let ctx = egui::Context::default();
    let mut state = GridState {
        quick_search_open: false,
        ..Default::default()
    };
    let mut columns = vec![Column::new(ColumnId(1), "SKU", 150.0)];
    business_frame(&ctx, &mut state, &mut columns, vec![]);
    business_click(&ctx, &mut state, &mut columns, Pos2::new(80.0, 53.0));
    let original = state.selection.regions().to_vec();
    let shift = egui::Modifiers {
        shift: true,
        ..Default::default()
    };
    let ctrl = egui::Modifiers {
        ctrl: true,
        command: true,
        ..Default::default()
    };
    for (pos, modifiers, expected) in [
        (Pos2::new(80.0, 137.0), shift, 4),
        (Pos2::new(80.0, 81.0), ctrl, 3),
        (Pos2::new(80.0, 81.0), ctrl, 4),
        (Pos2::new(80.0, 137.0), shift, 1),
    ] {
        business_frame(
            &ctx,
            &mut state,
            &mut columns,
            vec![
                egui::Event::ModifiersChanged(modifiers),
                egui::Event::PointerMoved(pos),
            ],
        );
        business_frame(
            &ctx,
            &mut state,
            &mut columns,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers,
            }],
        );
        // Holding a modifier must not toggle again each frame.
        for _ in 0..3 {
            business_frame(&ctx, &mut state, &mut columns, vec![]);
        }
        business_frame(
            &ctx,
            &mut state,
            &mut columns,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers,
            }],
        );
        assert_eq!(state.row_selection.checked().len(), expected);
        assert_eq!(state.selection.regions(), original);
        assert_eq!(state.row_selection.active, Some(RowId(1000)));
    }
}

fn business_frame(
    ctx: &egui::Context,
    state: &mut GridState,
    columns: &mut Vec<Column>,
    events: Vec<egui::Event>,
) {
    let events = events
        .into_iter()
        .map(|mut event| {
            match &mut event {
                egui::Event::PointerMoved(pos) | egui::Event::PointerButton { pos, .. }
                    if pos.x >= 44.0 =>
                {
                    pos.x += NUMBER_WIDTH
                }
                _ => {}
            }
            event
        })
        .collect();
    let mut frame = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            events,
            ..Default::default()
        },
        |root| {
            egui::CentralPanel::default().show(root, |ui| {
                DataGrid {
                    id: Id::new("business-test"),
                    columns,
                    rows: 100,
                    state,
                    row_id: Some(&|row| RowId(1000 + row as u128)),
                }
                .show(ui, |row, _| row.to_string());
            });
        },
    );
    frame.textures_delta.clear();
}

fn business_click(
    ctx: &egui::Context,
    state: &mut GridState,
    columns: &mut Vec<Column>,
    pos: Pos2,
) {
    business_frame(ctx, state, columns, vec![egui::Event::PointerMoved(pos)]);
    business_frame(ctx, state, columns, vec![pointer_button(pos, true)]);
    business_frame(ctx, state, columns, vec![pointer_button(pos, false)]);
}

#[test]
fn checkbox_active_row_and_copy_drag_are_independent() {
    let ctx = egui::Context::default();
    let mut columns = vec![
        Column::new(ColumnId(1), "SKU", 100.0),
        Column::new(ColumnId(2), "Name", 100.0),
    ];
    let mut state = GridState {
        quick_search_open: false,
        ..Default::default()
    };
    business_frame(&ctx, &mut state, &mut columns, vec![]);
    business_click(&ctx, &mut state, &mut columns, Pos2::new(70.0, 53.0));
    assert_eq!(state.row_selection.active, Some(RowId(1000)));
    assert!(state.row_selection.checked().is_empty());
    let selection = state.selection.regions().to_vec();
    business_click(&ctx, &mut state, &mut columns, Pos2::new(22.0, 81.0));
    assert_eq!(state.row_selection.targets(), [RowId(1001)]);
    assert_eq!(state.selection.regions(), selection);
    assert_eq!(state.row_selection.active, Some(RowId(1000)));
    let start = Pos2::new(70.0, 109.0);
    let end = Pos2::new(170.0, 193.0);
    business_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(start)],
    );
    business_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![pointer_button(start, true)],
    );
    business_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(end)],
    );
    business_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![pointer_button(end, false)],
    );
    assert_eq!(state.row_selection.active, Some(RowId(1000)));
    assert_eq!(state.row_selection.targets(), [RowId(1001)]);
    assert_eq!(state.selection.cell_count(), 8);
    business_click(&ctx, &mut state, &mut columns, Pos2::new(22.0, 81.0));
    assert_eq!(state.row_selection.targets(), [RowId(1000)]);
}

#[test]
fn search_input_does_not_select_or_move_a_column_and_remains_visible() {
    let ctx = egui::Context::default();
    let mut columns = vec![Column::new(ColumnId(1), "SKU", 150.0)];
    let mut state = GridState {
        quick_search_open: true,
        ..Default::default()
    };
    business_frame(&ctx, &mut state, &mut columns, vec![]);
    business_click(&ctx, &mut state, &mut columns, Pos2::new(80.0, 52.0));
    business_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::Text("SKU-1".into())],
    );
    assert_eq!(
        state.quick_search.get(&ColumnId(1)),
        Some(&erp_grid_core::FilterDraft::Text("SKU-1".into()))
    );
    assert!(state.gesture.is_none());
    assert!(state.row_selection.targets().is_empty());
    assert_eq!(state.selection.cell_count(), 0);
    // Search is a permanent row; the old collapse toggle no longer exists.
    assert!(state.quick_search_open);
    assert_eq!(
        state.quick_search[&ColumnId(1)],
        erp_grid_core::FilterDraft::Text("SKU-1".into())
    );
}

fn pointer_button(pos: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn shift_checkbox_sets_and_clears_a_range_without_changing_active_row() {
    let ctx = egui::Context::default();
    let mut state = GridState {
        quick_search_open: false,
        ..Default::default()
    };
    let mut columns = vec![Column::new(ColumnId(1), "SKU", 150.0)];
    business_frame(&ctx, &mut state, &mut columns, vec![]);
    business_click(&ctx, &mut state, &mut columns, Pos2::new(22.0, 53.0));
    let shift = egui::Modifiers {
        shift: true,
        ..Default::default()
    };
    let p = Pos2::new(22.0, 137.0);
    business_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![
            egui::Event::ModifiersChanged(shift),
            egui::Event::PointerMoved(p),
        ],
    );
    for pressed in [true, false] {
        business_frame(
            &ctx,
            &mut state,
            &mut columns,
            vec![egui::Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: shift,
            }],
        );
    }
    assert_eq!(state.row_selection.checked().len(), 4);
    let p = Pos2::new(22.0, 53.0);
    business_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(p)],
    );
    for pressed in [true, false] {
        business_frame(
            &ctx,
            &mut state,
            &mut columns,
            vec![egui::Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: shift,
            }],
        );
    }
    assert!(state.row_selection.checked().is_empty());
    assert!(state.row_selection.active.is_none());
    assert_eq!(state.selection.cell_count(), 0);
}

#[test]
fn column_resize_and_reorder_never_select_data() {
    let ctx = egui::Context::default();
    let normal = egui::Modifiers::NONE;
    let mut columns: Vec<_> = (0..3)
        .map(|id| Column::new(ColumnId(id), format!("C{id}"), 100.0))
        .collect();
    let mut state = GridState {
        quick_search_open: false,
        ..Default::default()
    };
    input_frame(&ctx, &mut state, &mut columns, vec![], normal);
    let edge = Pos2::new(105.0, 20.0);
    input_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(edge)],
        normal,
    );
    input_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![pointer_button(edge, true)],
        normal,
    );
    let resized = Pos2::new(145.0, 20.0);
    input_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(resized)],
        normal,
    );
    input_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![pointer_button(resized, false)],
        normal,
    );
    assert_eq!(columns[0].width(), 140.0);
    assert_eq!(state.selection.cell_count(), 0);
    let start = Pos2::new(40.0, 20.0);
    let end = Pos2::new(320.0, 20.0);
    input_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(start)],
        normal,
    );
    input_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![pointer_button(start, true)],
        normal,
    );
    input_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(end)],
        normal,
    );
    input_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![pointer_button(end, false)],
        normal,
    );
    assert_eq!(
        columns.iter().map(|c| c.id.0).collect::<Vec<_>>(),
        [1, 2, 0]
    );
    assert_eq!(state.selection.cell_count(), 0);
}

#[test]
fn view_change_clears_old_coordinate_selection() {
    let mut state = GridState {
        quick_search_open: false,
        ..Default::default()
    };
    state.bind_view(1);
    state
        .selection
        .apply(
            CellRect::between(Cell { row: 1, column: 2 }, Cell { row: 5, column: 3 }),
            SelectionOperation::Replace,
        )
        .unwrap();
    state.bind_view(1);
    assert_eq!(state.selection.cell_count(), 10);
    state.bind_view(2);
    assert_eq!(state.selection.cell_count(), 0);
}

#[test]
fn headless_frame_does_not_request_a_million_rows() {
    let context = egui::Context::default();
    let mut columns: Vec<_> = (0..100)
        .map(|id| Column::new(ColumnId(id), format!("C{id}"), 100.0))
        .collect();
    let mut state = GridState {
        quick_search_open: false,
        ..Default::default()
    };
    let mut painted = 0;
    let mut frame = context.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            ..Default::default()
        },
        |root| {
            egui::CentralPanel::default().show(root, |ui| {
                painted = DataGrid {
                    row_id: None,
                    id: Id::new("test"),
                    columns: &mut columns,
                    rows: 1_000_000,
                    state: &mut state,
                }
                .show(ui, |row, col| format!("{row}:{}", col.0))
                .painted_cells;
            });
        },
    );
    frame.textures_delta.clear();
    assert!(painted > 0 && painted < 300, "painted {painted} cells");
}

fn input_frame(
    context: &egui::Context,
    state: &mut GridState,
    columns: &mut Vec<Column>,
    mut events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
) {
    for event in &mut events {
        match event {
            egui::Event::PointerMoved(pos) | egui::Event::PointerButton { pos, .. } => {
                pos.x += STRUCTURE_WIDTH
            }
            _ => {}
        }
    }
    events.insert(0, egui::Event::ModifiersChanged(modifiers));
    let mut frame = context.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            events,
            ..Default::default()
        },
        |root| {
            egui::CentralPanel::default().show(root, |ui| {
                DataGrid {
                    row_id: None,
                    id: Id::new("gesture-test"),
                    columns,
                    rows: 100,
                    state,
                }
                .show(ui, |_, _| String::new());
            });
        },
    );
    frame.textures_delta.clear();
}

#[test]
fn modifier_without_row_identity_does_not_change_copy_selection() {
    let ctx = egui::Context::default();
    let mut columns = vec![
        Column::new(ColumnId(1), "A", 100.0),
        Column::new(ColumnId(2), "B", 100.0),
    ];
    let mut state = GridState {
        quick_search_open: false,
        ..Default::default()
    };
    let normal = egui::Modifiers::NONE;
    let ctrl = egui::Modifiers {
        ctrl: true,
        command: true,
        ..normal
    };
    let a = Pos2::new(40.0, 53.0);
    let b = Pos2::new(140.0, 109.0);
    input_frame(&ctx, &mut state, &mut columns, vec![], normal);
    input_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(a)],
        normal,
    );
    input_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerButton {
            pos: a,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: normal,
        }],
        normal,
    );
    input_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(b)],
        normal,
    );
    input_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerButton {
            pos: b,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: normal,
        }],
        normal,
    );
    assert_eq!(state.selection.cell_count(), 6);
    input_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(a)],
        ctrl,
    );
    input_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerButton {
            pos: a,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: ctrl,
        }],
        ctrl,
    );
    for _ in 0..4 {
        input_frame(&ctx, &mut state, &mut columns, vec![], ctrl);
    }
    assert_eq!(state.selection.cell_count(), 6);
    assert!(state.message.is_some());
    input_frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: normal,
        }],
        normal,
    );
    assert_eq!(
        state.selection.cell_count(),
        0,
        "escape clears copy selection when no row gesture was started"
    );
}

#[test]
fn quick_search_is_visible_by_default() {
    assert!(GridState::default().quick_search_open);
}

#[test]
fn read_only_header_checkbox_selects_and_inverts_page() {
    let ctx = egui::Context::default();
    let mut columns = vec![Column::new(ColumnId(1), "SKU", 120.0)];
    let mut state = GridState {
        read_only: true,
        quick_search_open: false,
        ..Default::default()
    };
    business_frame(&ctx, &mut state, &mut columns, vec![]);
    let pos = Pos2::new(22.0, 20.0);
    business_click(&ctx, &mut state, &mut columns, pos);
    assert_eq!(state.row_selection.checked().len(), 100);
    for pressed in [true, false] {
        business_frame(
            &ctx,
            &mut state,
            &mut columns,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    assert!(state.row_selection.checked().is_empty());
}
