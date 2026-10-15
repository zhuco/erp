use super::*;

#[test]
fn pending_editor_paints_only_the_frozen_draft_without_emitting_another_edit() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    let mut column = Column::new(ColumnId(1), "Qty", 220.0);
    column.edit_kind = erp_grid_core::EditKind::Integer;
    state
        .restore_edit(
            Cell { row: 0, column: 0 },
            RowId(1),
            ColumnId(1),
            "12345".into(),
            "777".into(),
        )
        .unwrap();
    let (_, ticket) = state.prepare_edit_commit(&column).unwrap().unwrap();
    let mut columns = vec![column];
    for _ in 0..2 {
        let mut grid = GridOutput::default();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 300.0))),
                ..Default::default()
            },
            |root| {
                egui::CentralPanel::default().show(root, |ui| {
                    grid = DataGrid {
                        id: Id::new("pending-paint-test"),
                        columns: &mut columns,
                        rows: 1,
                        state: &mut state,
                        row_id: Some(&|_| RowId(1)),
                    }
                    .show(ui, |_, _| "12345".into());
                });
            },
        );
        output.textures_delta.clear();
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.text()),
                _ => None,
            })
            .collect();
        assert_eq!(texts.iter().filter(|text| **text == "12345").count(), 0);
        assert_eq!(texts.iter().filter(|text| **text == "777").count(), 1);
        assert!(grid.editor_drawn);
        assert!(grid.edits.is_empty());
        assert_eq!(state.edit_draft().unwrap().after, "777");
        assert!(state.prepare_edit_commit(&columns[0]).unwrap().is_none());
    }
    assert!(state.resolve_edit(ticket, Ok(())));
}

#[test]
fn external_row_update_blocks_keyboard_and_host_commit_without_losing_draft() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    let mut column = Column::new(ColumnId(1), "Qty", 150.0);
    column.edit_kind = erp_grid_core::EditKind::Integer;
    let mut columns = vec![column];
    state
        .restore_edit(
            Cell { row: 0, column: 0 },
            RowId(1),
            ColumnId(1),
            "1".into(),
            "7".into(),
        )
        .unwrap();
    frame(&ctx, &mut state, &mut columns, vec![]);
    assert!(!state.notify_row_changed(RowId(2)));
    assert!(state.notify_row_changed(RowId(1)));
    assert!(state.prepare_edit_commit(&columns[0]).is_err());
    let output = frame(&ctx, &mut state, &mut columns, vec![key(Key::Enter)]);
    assert!(output.edits.is_empty());
    let draft = state.edit_draft().unwrap();
    assert_eq!(draft.before, "1");
    assert_eq!(draft.after, "7");
    assert!(state.rebase_edit(RowId(1), ColumnId(1), "2".into()));
    let (edit, ticket) = state.prepare_edit_commit(&columns[0]).unwrap().unwrap();
    assert_eq!(edit.before, "2");
    assert_eq!(edit.after, "7");
    assert!(!state.notify_row_changed(RowId(1)));
    assert!(state.resolve_edit(ticket, Ok(())));
}
#[test]
fn page_rebinding_keeps_only_business_checks_and_horizontal_scroll() {
    let mut state = GridState::default();
    state.bind_view(1);
    state.row_selection.toggle(RowId(101));
    state.row_selection.active = Some(RowId(102));
    state.focus = Some(Cell { row: 1, column: 1 });
    state.scroll = Vec2::new(125.0, 350.0);
    state.adopt_generation(2);
    assert_eq!(state.focus, Some(Cell { row: 1, column: 1 }));
    assert_eq!(state.scroll_offset(), Vec2::new(125.0, 350.0));
    state.rebind_page(3, true);
    assert_eq!(state.row_selection.targets(), [RowId(101)]);
    assert_eq!(state.row_selection.active, None);
    assert_eq!(state.focus, None);
    assert_eq!(state.scroll_offset(), Vec2::new(125.0, 0.0));
    state.bind_view(4);
    assert!(state.row_selection.checked().is_empty());
}

#[test]
fn restored_edit_is_reviewable_and_cannot_replace_an_active_editor() {
    let mut state = GridState::default();
    let cell = Cell { row: 2, column: 1 };
    state
        .restore_edit(cell, RowId(7), ColumnId(13), "2".into(), "3".into())
        .unwrap();
    assert_eq!(state.edit_draft().unwrap().after, "3");
    assert_eq!(state.focus, Some(cell));
    assert!(
        state
            .restore_edit(cell, RowId(8), ColumnId(13), "1".into(), "4".into())
            .is_err()
    );
}
fn frame(
    ctx: &egui::Context,
    state: &mut GridState,
    columns: &mut Vec<Column>,
    events: Vec<egui::Event>,
) -> GridOutput {
    let mut grid = GridOutput::default();
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            events,
            ..Default::default()
        },
        |root| {
            egui::CentralPanel::default().show(root, |ui| {
                grid = DataGrid {
                    id: Id::new("edit-test"),
                    columns,
                    rows: 10,
                    state,
                    row_id: Some(&|row| RowId(row as u128 + 1)),
                }
                .show(ui, |_, _| "1".into());
            });
        },
    );
    output.textures_delta.clear();
    grid
}
fn key(key: Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}
fn start(ctx: &egui::Context, state: &mut GridState, columns: &mut Vec<Column>) {
    let pos = Pos2::new(140.0, 80.0);
    frame(ctx, state, columns, vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        frame(
            ctx,
            state,
            columns,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    frame(ctx, state, columns, vec![key(Key::F2)]);
    frame(ctx, state, columns, vec![]);
}
#[test]
fn inline_editor_emits_valid_changes_and_escape_discards_invalid_drafts() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    let mut column = Column::new(ColumnId(1), "Qty", 150.0);
    column.edit_kind = erp_grid_core::EditKind::Integer;
    let mut columns = vec![column];
    frame(&ctx, &mut state, &mut columns, vec![]);
    start(&ctx, &mut state, &mut columns);
    assert!(state.is_editing());
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::Text("2".into())],
    );
    let output = frame(&ctx, &mut state, &mut columns, vec![key(Key::Enter)]);
    assert_eq!(output.edits.len(), 1);
    assert_eq!(output.edits[0].row, RowId(1));
    assert_eq!(output.edits[0].before, "1");
    assert_eq!(output.edits[0].after, "12");
    assert!(state.is_editing());
    assert!(
        frame(&ctx, &mut state, &mut columns, vec![key(Key::Enter)])
            .edits
            .is_empty()
    );
    assert!(state.resolve_edit(output.edit_ticket.unwrap(), Ok(())));
    assert!(!state.is_editing());
    start(&ctx, &mut state, &mut columns);
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::Text("x".into())],
    );
    let output = frame(&ctx, &mut state, &mut columns, vec![key(Key::Enter)]);
    assert!(output.edits.is_empty());
    assert!(state.is_editing());
    assert!(state.message.is_some());
    let output = frame(&ctx, &mut state, &mut columns, vec![key(Key::Tab)]);
    assert!(output.edits.is_empty());
    assert!(output.edit_navigation.is_none());
    assert!(state.is_editing());
    let output = frame(&ctx, &mut state, &mut columns, vec![key(Key::Escape)]);
    assert!(output.edits.is_empty());
    assert!(!state.is_editing());
}

#[test]
fn host_save_prepares_the_same_validated_ticket_and_waits_for_ack() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    let mut column = Column::new(ColumnId(1), "Qty", 150.0);
    column.edit_kind = erp_grid_core::EditKind::Integer;
    let mut columns = vec![column];
    frame(&ctx, &mut state, &mut columns, vec![]);
    start(&ctx, &mut state, &mut columns);
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::Text("2".into())],
    );
    let wrong = Column::new(ColumnId(2), "Other", 100.0);
    assert!(state.prepare_edit_commit(&wrong).is_err());
    let (edit, ticket) = state.prepare_edit_commit(&columns[0]).unwrap().unwrap();
    assert_eq!(edit.after, "12");
    assert!(state.prepare_edit_commit(&columns[0]).unwrap().is_none());
    assert!(state.resolve_edit(ticket, Ok(())));
    assert!(!state.is_editing());
}

#[test]
fn leave_hold_prevents_focus_loss_from_submitting_the_draft() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    let mut column = Column::new(ColumnId(1), "Qty", 150.0);
    column.edit_kind = erp_grid_core::EditKind::Integer;
    let mut columns = vec![column];
    frame(&ctx, &mut state, &mut columns, vec![]);
    start(&ctx, &mut state, &mut columns);
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::Text("2".into())],
    );
    state.hold_edit();
    let pos = Pos2::new(700.0, 20.0);
    let output = frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    assert!(output.edits.is_empty());
    assert_eq!(state.edit_draft().unwrap().after, "12");
}

#[test]
fn tab_emits_direction_and_failed_save_preserves_draft_without_duplicate_submission() {
    for backwards in [false, true] {
        let ctx = egui::Context::default();
        let mut state = GridState::default();
        let mut column = Column::new(ColumnId(1), "Qty", 150.0);
        column.edit_kind = erp_grid_core::EditKind::Integer;
        let mut columns = vec![column];
        frame(&ctx, &mut state, &mut columns, vec![]);
        start(&ctx, &mut state, &mut columns);
        frame(
            &ctx,
            &mut state,
            &mut columns,
            vec![egui::Event::Text("2".into())],
        );
        let mut tab = key(Key::Tab);
        if let egui::Event::Key { modifiers, .. } = &mut tab {
            modifiers.shift = backwards;
        }
        let output = frame(&ctx, &mut state, &mut columns, vec![tab]);
        assert_eq!(
            output.edit_navigation,
            Some(EditNavigation {
                row: RowId(1),
                column: ColumnId(1),
                backwards
            })
        );
        assert_eq!(output.edits.len(), 1);
        let ticket = output.edit_ticket.unwrap();
        assert!(!state.resolve_edit(ticket.wrapping_add(1), Ok(())));
        let output = frame(&ctx, &mut state, &mut columns, vec![key(Key::Tab)]);
        assert!(output.edits.is_empty());
        assert!(output.edit_navigation.is_none());
        state.cancel_edit();
        assert!(
            state.is_editing(),
            "pending save cannot be discarded as an unsent draft"
        );
        assert!(state.resolve_edit(ticket, Err("Version conflict".into())));
        frame(&ctx, &mut state, &mut columns, vec![]);
        assert_eq!(state.editor.as_ref().unwrap().draft, "12");
        assert_eq!(state.editor.as_ref().unwrap().before, "1");
        let retried = frame(&ctx, &mut state, &mut columns, vec![key(Key::Enter)]);
        let new_ticket = retried.edit_ticket.unwrap();
        assert_ne!(new_ticket, ticket);
        assert!(
            !state.resolve_edit(ticket, Ok(())),
            "late response cannot acknowledge a new attempt"
        );
        assert!(state.resolve_edit(new_ticket, Err("Still conflicting".into())));
        frame(&ctx, &mut state, &mut columns, vec![]);
        frame(&ctx, &mut state, &mut columns, vec![key(Key::Escape)]);
        assert!(!state.is_editing());
    }
}
#[test]
fn paste_is_routed_to_grid_focus_without_mutating_rows() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    let mut columns = vec![Column::new(ColumnId(1), "Name", 150.0)];
    frame(&ctx, &mut state, &mut columns, vec![]);
    let pos = Pos2::new(140.0, 80.0);
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
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    let output = frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::Paste("a\tb".into())],
    );
    assert_eq!(
        output.paste,
        Some((Cell { row: 0, column: 0 }, "a\tb".into()))
    );
    assert!(output.edits.is_empty());
}

#[test]
fn hidden_or_offscreen_editors_keep_invalid_drafts_and_can_cancel() {
    for hidden in [false, true] {
        let ctx = egui::Context::default();
        let mut state = GridState::default();
        let mut columns: Vec<_> = (1..=10)
            .map(|id| Column::new(ColumnId(id), "Quantity", 150.0))
            .collect();
        columns[0].edit_kind = erp_grid_core::EditKind::Integer;
        frame(&ctx, &mut state, &mut columns, vec![]);
        start(&ctx, &mut state, &mut columns);
        frame(
            &ctx,
            &mut state,
            &mut columns,
            vec![egui::Event::Text("x".into())],
        );
        if hidden {
            columns[0].visible = false;
        } else {
            state.scroll.x = 800.0;
        }
        frame(&ctx, &mut state, &mut columns, vec![]);
        assert!(state.is_editing());
        let output = frame(&ctx, &mut state, &mut columns, vec![key(Key::Enter)]);
        assert!(output.edits.is_empty());
        assert!(state.is_editing());
        assert_eq!(state.editor.as_ref().unwrap().draft, "1x");
        let output = frame(&ctx, &mut state, &mut columns, vec![key(Key::Escape)]);
        assert!(output.edits.is_empty());
        assert!(!state.is_editing());
    }
}
