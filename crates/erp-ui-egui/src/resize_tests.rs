use super::*;

fn frame(
    ctx: &egui::Context,
    state: &mut GridState,
    columns: &mut Vec<Column>,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            events,
            ..Default::default()
        },
        |root| {
            egui::CentralPanel::default().show(root, |ui| {
                DataGrid {
                    id: Id::new("resize-regression"),
                    columns,
                    state,
                    rows: 100,
                    row_id: Some(&|row| RowId(row as u128)),
                }
                .show(ui, |_, _| "Value".into());
            });
        },
    );
    output.textures_delta.clear();
    output
}
fn button(pos: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    }
}

#[test]
fn remote_capabilities_disable_sort_and_column_filter_without_disabling_grid() {
    let ctx = egui::Context::default();
    let mut state = GridState {
        sorting_enabled: false,
        filtering_enabled: false,
        ..Default::default()
    };
    let mut columns = vec![Column::new(ColumnId(1), "Product", 200.0)];
    frame(&ctx, &mut state, &mut columns, vec![]);
    for point in [Pos2::new(150.0, 22.0), Pos2::new(150.0, 52.0)] {
        frame(
            &ctx,
            &mut state,
            &mut columns,
            vec![egui::Event::PointerMoved(point)],
        );
        for pressed in [true, false] {
            frame(&ctx, &mut state, &mut columns, vec![button(point, pressed)]);
        }
    }
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::Text("ignored".into())],
    );
    assert!(state.sorts.is_empty());
    assert!(
        matches!(state.quick_search.get(&ColumnId(1)), Some(erp_grid_core::FilterDraft::Text(value)) if value.is_empty())
    );
    state.row_selection.toggle(RowId(1));
    assert_eq!(state.row_selection.targets(), [RowId(1)]);
}

#[test]
fn divider_double_click_sizes_content_without_sorting_or_losing_selection() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    let mut columns = vec![
        Column::new(ColumnId(1), "A", 300.0),
        Column::new(ColumnId(2), "B", 120.0),
    ];
    frame(&ctx, &mut state, &mut columns, vec![]);
    let cell = Cell { row: 1, column: 0 };
    state.focus = Some(cell);
    state
        .selection
        .apply(CellRect::between(cell, cell), SelectionOperation::Replace)
        .unwrap();
    let point = Pos2::new(CHECK_WIDTH + NUMBER_WIDTH + 8.0 + 300.0, 22.0);
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(point)],
    );
    for _ in 0..2 {
        frame(&ctx, &mut state, &mut columns, vec![button(point, true)]);
        frame(&ctx, &mut state, &mut columns, vec![button(point, false)]);
    }
    assert!(
        (64.0..120.0).contains(&columns[0].width()),
        "{}",
        columns[0].width()
    );
    assert_eq!(columns[1].width(), 120.0);
    assert!(state.sorts.is_empty());
    assert_eq!(state.focus, Some(cell));
    assert!(state.selection.contains(cell));
}

#[test]
fn either_side_of_divider_resizes_same_column_without_sorting_or_selection() {
    for delta in [-3.0, 0.0, 3.0] {
        let ctx = egui::Context::default();
        let mut state = GridState::default();
        let mut columns = vec![
            Column::new(ColumnId(1), "A", 120.0),
            Column::new(ColumnId(2), "B", 120.0),
        ];
        frame(&ctx, &mut state, &mut columns, vec![]);
        let start = Pos2::new(CHECK_WIDTH + NUMBER_WIDTH + 8.0 + 120.0 + delta, 22.0);
        let hover = frame(
            &ctx,
            &mut state,
            &mut columns,
            vec![egui::Event::PointerMoved(start)],
        );
        assert_eq!(
            hover.platform_output.cursor_icon,
            egui::CursorIcon::ResizeHorizontal
        );
        frame(&ctx, &mut state, &mut columns, vec![button(start, true)]);
        let end = start + Vec2::new(60.0, 0.0);
        frame(
            &ctx,
            &mut state,
            &mut columns,
            vec![egui::Event::PointerMoved(end)],
        );
        frame(&ctx, &mut state, &mut columns, vec![button(end, false)]);
        assert_eq!(columns[0].width(), 180.0);
        assert_eq!(columns[1].width(), 120.0);
        assert_eq!(columns[0].id, ColumnId(1));
        assert!(state.sorts.is_empty());
        assert!(state.selection.regions().is_empty());
        assert!(state.row_selection.targets().is_empty());
        // A click without a drag on the divider must not sort either neighbor.
        let point = Pos2::new(CHECK_WIDTH + NUMBER_WIDTH + 8.0 + 180.0, 22.0);
        frame(
            &ctx,
            &mut state,
            &mut columns,
            vec![egui::Event::PointerMoved(point)],
        );
        frame(&ctx, &mut state, &mut columns, vec![button(point, true)]);
        frame(&ctx, &mut state, &mut columns, vec![button(point, false)]);
        assert!(state.sorts.is_empty());
    }
}

#[test]
fn frozen_edge_after_horizontal_scroll_resizes_and_escape_restores_original_width() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    let mut columns: Vec<_> = (1..=12)
        .map(|id| Column::new(ColumnId(id), format!("C{id}"), 120.0))
        .collect();
    columns[0].pinned = true;
    for _ in 0..3 {
        frame(&ctx, &mut state, &mut columns, vec![]);
    }
    state.scroll.x = 240.0;
    frame(&ctx, &mut state, &mut columns, vec![]);
    let start = Pos2::new(8.0 + CHECK_WIDTH + NUMBER_WIDTH + 122.0, 22.0);
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(start)],
    );
    frame(&ctx, &mut state, &mut columns, vec![button(start, true)]);
    let end = start + Vec2::new(50.0, 0.0);
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::PointerMoved(end)],
    );
    assert_eq!(columns[0].width(), 170.0);
    frame(
        &ctx,
        &mut state,
        &mut columns,
        vec![egui::Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }],
    );
    frame(&ctx, &mut state, &mut columns, vec![button(end, false)]);
    assert_eq!(columns[0].width(), 120.0);
    assert!(columns[0].pinned);
    assert!(state.sorts.is_empty());
}

#[test]
fn explicit_width_configuration_preserves_selection_and_theme_override_is_painted() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    let mut columns = vec![Column::new(ColumnId(1), "A", 120.0)];
    frame(&ctx, &mut state, &mut columns, vec![]);
    let cell = Cell { row: 0, column: 0 };
    state
        .selection
        .apply(CellRect::between(cell, cell), SelectionOperation::Replace)
        .unwrap();
    state.focus = Some(cell);
    state.column_action = Some(column_config::ColumnAction::Width(ColumnId(1), 256.0));
    frame(&ctx, &mut state, &mut columns, vec![]);
    assert_eq!(columns[0].width(), 256.0);
    assert!(state.selection.contains(cell));
    assert_eq!(state.focus, Some(cell));
    let color = egui::Color32::from_rgb(200, 100, 30);
    let mut theme = GridTheme::from_visuals(&egui::Visuals::light());
    theme.selected_cell = color;
    state.theme = Some(theme);
    let output = frame(&ctx, &mut state, &mut columns, vec![]);
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::epaint::Shape::Rect(rect) if rect.fill == color && rect.rect.intersects(shape.clip_rect))));
}

#[test]
fn configured_width_bounds_cover_drag_double_click_config_and_preference_restore() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    let mut columns = vec![Column::new(ColumnId(1), "A", 120.0).with_width_bounds(96.0, 180.0)];
    frame(&ctx, &mut state, &mut columns, vec![]);

    // A configuration value beyond the business limit is clamped by Column.
    state.column_action = Some(column_config::ColumnAction::Width(ColumnId(1), 999.0));
    frame(&ctx, &mut state, &mut columns, vec![]);
    assert_eq!(columns[0].width(), 180.0);

    // Preference restore goes through the same resize contract.
    let preferences = erp_grid_core::ViewPreferences {
        version: 1,
        scope: "test".into(),
        columns: vec![erp_grid_core::ColumnPreference {
            id: ColumnId(1),
            width: 1.0,
            visible: true,
            pinned: false,
            pinned_right: false,
        }],
        filters: vec![],
        sorts: vec![],
        page_size: 20,
        quick_search_open: true,
        search: None,
        expression: None,
    };
    preferences.apply_columns("test", &mut columns).unwrap();
    assert_eq!(columns[0].width(), 96.0);
}
