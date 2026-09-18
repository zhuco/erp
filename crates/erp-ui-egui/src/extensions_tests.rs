use super::*;

fn frame(
    ctx: &egui::Context,
    state: &mut GridState,
    columns: &mut Vec<Column>,
    events: Vec<egui::Event>,
    present: Option<&dyn Fn(CellContext<'_>) -> CellPresentation>,
) -> (GridOutput, Vec<(String, Pos2)>) {
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
                    id: Id::new("audit-extensions"),
                    columns,
                    state,
                    rows: 10,
                    row_id: Some(&|row| RowId(row as u128 + 1)),
                }
                .show_with_options(
                    ui,
                    |_, _| "raw".into(),
                    GridOptions {
                        present,
                        ..Default::default()
                    },
                    |_, _| {},
                );
            });
        },
    );
    let labels = output
        .shapes
        .iter()
        .filter_map(|shape| {
            if let egui::epaint::Shape::Text(text) = &shape.shape {
                Some((
                    text.galley.text().to_owned(),
                    text.pos + text.galley.size() * 0.5,
                ))
            } else {
                None
            }
        })
        .collect();
    output.textures_delta.clear();
    (grid, labels)
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
fn pointer(pos: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        pressed,
        button: egui::PointerButton::Primary,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn audit_entity_search_rejects_stale_results_and_commits_selected_identity() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    let mut column = Column::new(ColumnId(1), "SKU", 220.0);
    column.edit_kind = erp_grid_core::EditKind::Entity;
    let mut columns = vec![column];
    state
        .restore_edit(
            Cell { row: 0, column: 0 },
            RowId(1),
            ColumnId(1),
            RowId(10).to_uuid(),
            RowId(11).to_uuid(),
        )
        .unwrap();
    frame(&ctx, &mut state, &mut columns, vec![], None);
    let first = state.take_entity_search().unwrap();
    let mut stale = first.clone();
    stale.revision += 1;
    let choice = EntityOption {
        id: RowId(99),
        label: "Searchable SKU".into(),
    };
    assert!(!state.resolve_entity_options(&stale, Ok(vec![choice.clone()])));
    assert!(state.prepare_edit_commit(&columns[0]).is_err());
    assert!(state.resolve_entity_options(&first, Ok(vec![choice])));
    assert!(!state.resolve_entity_options(&first, Ok(vec![])));
    frame(&ctx, &mut state, &mut columns, vec![], None);
    let selection = frame(&ctx, &mut state, &mut columns, vec![key(Key::Enter)], None).0;
    assert!(
        selection.edits.is_empty(),
        "first Enter chooses, it does not save"
    );
    let saved = frame(&ctx, &mut state, &mut columns, vec![key(Key::Enter)], None).0;
    assert_eq!(saved.edits.len(), 1);
    assert_eq!(saved.edits[0].after, RowId(99).to_uuid());
    let ticket = saved.edit_ticket.unwrap();
    assert!(state.resolve_edit(ticket, Err("Conflict".into())));
    assert_eq!(state.edit_draft().unwrap().after, RowId(99).to_uuid());
    state.cancel_edit();
    assert!(!state.resolve_entity_options(&first, Ok(vec![])));
}

#[test]
fn audit_entity_ime_debounce_and_invalid_options_do_not_replace_selection() {
    let mut picker = entity_picker::Picker::default();
    let request = EntitySearch {
        instance: Id::new("one"),
        ticket: 1,
        row: RowId(1),
        column: ColumnId(1),
        revision: 0,
        search: String::new(),
    };
    picker.query = "商品".into();
    picker.invalidate(1.0);
    picker.poll(request.clone(), 2.0, true);
    assert!(picker.outgoing.is_none());
    picker.poll(request.clone(), 2.0, false);
    let old = picker.outgoing.take().unwrap();
    picker.query = "货品".into();
    picker.invalidate(2.0);
    assert!(!picker.resolve(&old, Ok(vec![])));
    picker.poll(request.clone(), 2.1, false);
    assert!(picker.outgoing.is_none());
    picker.poll(request, 2.3, false);
    let fresh = picker.outgoing.take().unwrap();
    assert_eq!(fresh.search, "货品");
    let choice = EntityOption {
        id: RowId(9),
        label: "SKU".into(),
    };
    assert!(picker.resolve(&fresh, Ok(vec![choice.clone(), choice])));
    assert!(picker.options.is_empty());
    assert!(picker.error.is_some());
    assert!(picker.choose(0).is_none());
}

#[test]
fn audit_cell_action_uses_stable_identity_and_does_not_change_selection() {
    let ctx = egui::Context::default();
    let mut state = GridState {
        quick_search_open: false,
        ..Default::default()
    };
    let mut columns = vec![Column::new(ColumnId(1), "Document", 180.0)];
    let present = |_: CellContext<'_>| CellPresentation::Action {
        id: "open".into(),
        label: "Open".into(),
        enabled: true,
    };
    frame(&ctx, &mut state, &mut columns, vec![], Some(&present));
    state.row_selection.set_checked(RowId(5), true);
    state
        .selection
        .apply(
            CellRect {
                top: 2,
                bottom: 3,
                left: 0,
                right: 1,
            },
            SelectionOperation::Replace,
        )
        .unwrap();
    let before = state.selection.regions().to_vec();
    let (_, labels) = frame(&ctx, &mut state, &mut columns, vec![], Some(&present));
    let pos = labels.iter().find(|(text, _)| text == "Open").unwrap().1;
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(pos)],
        Some(&present),
    );
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![pointer(pos, true)],
        Some(&present),
    );
    let output = frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![pointer(pos, false)],
        Some(&present),
    )
    .0;
    assert_eq!(
        output.cell_action,
        Some(CellAction {
            row: RowId(1),
            column: ColumnId(1),
            action: "open".into()
        })
    );
    assert_eq!(state.selection.regions(), before);
    assert_eq!(
        state
            .row_selection
            .checked()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![RowId(5)]
    );
    assert!(!state.is_editing());
    for pressed in [true, false] {
        let mut event = pointer(pos, pressed);
        if let egui::Event::PointerButton { modifiers, .. } = &mut event {
            *modifiers = egui::Modifiers::CTRL;
        }
        let output = frame(
            &ctx,
            &mut state,
            &mut columns,
            vec![egui::Event::ModifiersChanged(egui::Modifiers::CTRL), event],
            Some(&present),
        )
        .0;
        assert!(
            output.cell_action.is_none(),
            "Ctrl click checks the row instead of activating a cell action"
        );
    }
    assert!(state.row_selection.checked().contains(&RowId(1)));
    assert_eq!(state.selection.regions(), before);
}

#[test]
fn audit_presentation_keeps_raw_copy_and_choice_picker_is_rendered() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    let mut column = Column::new(ColumnId(1), "Status", 220.0);
    column.edit_kind = erp_grid_core::EditKind::Choice(vec!["Ready".into(), "Hold".into()]);
    let mut columns = vec![column];
    let present = |_: CellContext<'_>| CellPresentation::Badge {
        label: "Displayed".into(),
        color: egui::Color32::DARK_BLUE,
    };
    let (_, labels) = frame(&ctx, &mut state, &mut columns, vec![], Some(&present));
    assert!(labels.iter().any(|(text, _)| text == "Displayed"));
    state
        .selection
        .apply(
            CellRect {
                top: 0,
                bottom: 1,
                left: 0,
                right: 1,
            },
            SelectionOperation::Replace,
        )
        .unwrap();
    assert_eq!(
        selection_tsv(&state.selection, &columns, 10, |_, _| "raw".into()).unwrap(),
        "raw"
    );
    state
        .restore_edit(
            Cell { row: 0, column: 0 },
            RowId(1),
            ColumnId(1),
            "Ready".into(),
            "Hold".into(),
        )
        .unwrap();
    frame(&ctx, &mut state, &mut columns, vec![], None);
    let (_, labels) = frame(&ctx, &mut state, &mut columns, vec![], None);
    let pos = labels.iter().find(|(text, _)| text == "▾").unwrap().1;
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(pos), pointer(pos, true)],
        None,
    );
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![pointer(pos, false)],
        None,
    );
    let (_, labels) = frame(&ctx, &mut state, &mut columns, vec![], None);
    assert!(labels.iter().any(|(text, _)| text == "Ready"));
    assert!(state.is_editing(), "opening picker must not auto-save");
    let option = labels.iter().find(|(text, _)| text == "Ready").unwrap().1;
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(option), pointer(option, true)],
        None,
    );
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![pointer(option, false)],
        None,
    );
    assert_eq!(state.edit_draft().unwrap().after, "Ready");
}

#[test]
fn audit_date_calendar_changes_draft_then_uses_the_existing_save_ticket() {
    let ctx = egui::Context::default();
    let mut state = GridState {
        quick_search_open: false,
        ..Default::default()
    };
    let mut column = Column::new(ColumnId(1), "Date", 220.0);
    column.edit_kind = erp_grid_core::EditKind::Date;
    let mut columns = vec![column];
    state
        .restore_edit(
            Cell { row: 0, column: 0 },
            RowId(1),
            ColumnId(1),
            "2026-02-27".into(),
            "2026-02-28".into(),
        )
        .unwrap();
    frame(&ctx, &mut state, &mut columns, vec![], None);
    let (_, labels) = frame(&ctx, &mut state, &mut columns, vec![], None);
    let selector = labels.iter().find(|(text, _)| text == "▾").unwrap().1;
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(selector), pointer(selector, true)],
        None,
    );
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![pointer(selector, false)],
        None,
    );
    let (_, labels) = frame(&ctx, &mut state, &mut columns, vec![], None);
    let day = labels.iter().find(|(text, _)| text == "01").unwrap().1;
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(day), pointer(day, true)],
        None,
    );
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![pointer(day, false)],
        None,
    );
    assert_eq!(state.edit_draft().unwrap().after, "2026-02-01");
    let (edit, ticket) = state.prepare_edit_commit(&columns[0]).unwrap().unwrap();
    assert_eq!(edit.before, "2026-02-27");
    assert_eq!(edit.after, "2026-02-01");
    assert!(state.resolve_edit(ticket, Ok(())));
}

#[test]
fn audit_images_use_host_texture_and_only_visit_the_viewport() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    let mut columns = vec![Column::new(ColumnId(1), "Image", 120.0)];
    let visits = std::cell::Cell::new(0usize);
    let mut painted = 0;
    let present = |_: CellContext<'_>| {
        visits.set(visits.get() + 1);
        CellPresentation::Image {
            texture: egui::TextureId::User(42),
            tooltip: "Product image".into(),
        }
    };
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            ..Default::default()
        },
        |ui| {
            painted = DataGrid {
                id: Id::new("image-audit"),
                columns: &mut columns,
                state: &mut state,
                rows: 1_000_000,
                row_id: Some(&|row| RowId(row as u128 + 1)),
            }
            .show_with_options(
                ui,
                |_, _| "image-key".into(),
                GridOptions {
                    present: Some(&present),
                    ..Default::default()
                },
                |_, _| {},
            )
            .painted_cells;
        },
    );
    assert!(painted > 0 && painted < 40);
    assert!(visits.get() < 100);
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,egui::epaint::Shape::Mesh(mesh) if mesh.texture_id==egui::TextureId::User(42))));
    output.textures_delta.clear();
}
