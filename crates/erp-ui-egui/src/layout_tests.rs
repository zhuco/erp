use super::*;

fn run(
    ctx: &egui::Context,
    state: &mut GridState,
    expanded: &[(usize, RowId)],
    events: Vec<egui::Event>,
) -> (egui::FullOutput, GridOutput) {
    let mut result = GridOutput::default();
    let mut columns: Vec<_> = (0..12)
        .map(|i| Column::new(ColumnId(i), format!("C{i}"), 120.0))
        .collect();
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            events,
            ..Default::default()
        },
        |root| {
            egui::CentralPanel::default().show(root, |ui| {
                result = DataGrid {
                    id: Id::new("layout-regression"),
                    columns: &mut columns,
                    rows: 100,
                    state,
                    row_id: Some(&|row| RowId(row as u128 + 1)),
                }
                .show_with_details(
                    ui,
                    |row, col| format!("{}:{row}", col.0),
                    true,
                    expanded,
                    |ui, key| {
                        ui.label(format!("Detail {}", key.0));
                    },
                );
            });
        },
    );
    output.textures_delta.clear();
    (output, result)
}

#[test]
fn serial_is_inside_actual_clip_and_full_content_has_no_phantom_tail() {
    let ctx = egui::Context::default();
    ctx.style_mut_of(egui::Theme::Dark, |style| style.animation_time = 0.0);
    ctx.style_mut_of(egui::Theme::Light, |style| style.animation_time = 0.0);
    let mut state = GridState::default();
    for _ in 0..3 {
        run(&ctx, &mut state, &[], vec![]);
    }
    let (output, metrics) = run(&ctx, &mut state, &[], vec![]);
    for label in ["序", "1"] {
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::epaint::Shape::Text(text) if text.galley.text() == label && shape.clip_rect.contains(text.pos + text.galley.size() / 2.0))), "{label} must be in the actual paint clip");
    }
    assert!(
        (metrics.content_height - 2860.0).abs() < 1.0,
        "unwanted bottom content: {}",
        metrics.content_height
    );
    state.scroll.y = metrics.content_height - metrics.viewport_height;
    for _ in 0..4 {
        run(&ctx, &mut state, &[], vec![]);
    }
    let (output, metrics) = run(&ctx, &mut state, &[], vec![]);
    let last = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) if text.galley.text() == "0:99" => {
                Some(text.pos.y + text.galley.size().y / 2.0)
            }
            _ => None,
        })
        .expect("last row painted");
    assert!(
        (last + ROW / 2.0 - (8.0 + metrics.viewport_height)).abs() < 2.0,
        "last row bottom must touch viewport bottom: {last}, {metrics:?}"
    );
}

#[test]
fn expansion_toggle_and_detail_hit_do_not_check_or_select_parent() {
    let ctx = egui::Context::default();
    ctx.style_mut_of(egui::Theme::Dark, |style| style.animation_time = 0.0);
    ctx.style_mut_of(egui::Theme::Light, |style| style.animation_time = 0.0);
    let mut state = GridState::default();
    for _ in 0..3 {
        run(&ctx, &mut state, &[], vec![]);
    }
    let pos = Pos2::new(52.0, 80.0);
    run(&ctx, &mut state, &[], vec![egui::Event::PointerMoved(pos)]);
    run(
        &ctx,
        &mut state,
        &[],
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        }],
    );
    let (_, output) = run(
        &ctx,
        &mut state,
        &[],
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }],
    );
    assert_eq!(output.toggle_detail, Some(RowId(1)));
    assert!(state.row_selection.targets().is_empty());
    assert!(state.selection.regions().is_empty());
    let (_, metrics) = run(&ctx, &mut state, &[(0, RowId(1))], vec![]);
    assert!((metrics.content_height - 3120.0).abs() < 1.0);
    let pos = Pos2::new(150.0, 180.0);
    run(
        &ctx,
        &mut state,
        &[(0, RowId(1))],
        vec![egui::Event::PointerMoved(pos)],
    );
    for pressed in [true, false] {
        run(
            &ctx,
            &mut state,
            &[(0, RowId(1))],
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            }],
        );
    }
    assert!(state.selection.regions().is_empty());
    assert!(state.row_selection.targets().is_empty());
    let pos = Pos2::new(150.0, 368.0);
    run(
        &ctx,
        &mut state,
        &[(0, RowId(1))],
        vec![egui::Event::PointerMoved(pos)],
    );
    for pressed in [true, false] {
        run(
            &ctx,
            &mut state,
            &[(0, RowId(1))],
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            }],
        );
    }
    assert_eq!(state.row_selection.active, Some(RowId(2)));
}

#[test]
fn tree_toggle_hit_emits_only_tree_intent() {
    let ctx = egui::Context::default();
    ctx.style_mut_of(egui::Theme::Dark, |style| style.animation_time = 0.0);
    ctx.style_mut_of(egui::Theme::Light, |style| style.animation_time = 0.0);
    let mut state = GridState::default();
    state.tree_rows.insert(
        RowId(1),
        TreeRowVisual {
            has_children: true,
            ..Default::default()
        },
    );
    for _ in 0..3 {
        run(&ctx, &mut state, &[], vec![]);
    }
    let pos = Pos2::new(CHECK_WIDTH + NUMBER_WIDTH + 42.0, 86.0);
    run(&ctx, &mut state, &[], vec![egui::Event::PointerMoved(pos)]);
    run(
        &ctx,
        &mut state,
        &[],
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        }],
    );
    let (_, output) = run(
        &ctx,
        &mut state,
        &[],
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }],
    );
    assert_eq!(output.toggle_tree, Some(RowId(1)));
    assert_eq!(output.toggle_detail, None);
    assert!(state.selection.regions().is_empty());
    assert!(state.row_selection.targets().is_empty());
    assert_eq!(state.row_selection.active, None);
}

#[test]
fn tree_parent_consumes_horizontal_key_before_cell_navigation() {
    let ctx = egui::Context::default();
    let mut state = GridState::default();
    state.tree_rows.insert(
        RowId(1),
        TreeRowVisual {
            has_children: true,
            ..Default::default()
        },
    );
    for _ in 0..3 {
        run(&ctx, &mut state, &[], vec![]);
    }
    let pos = Pos2::new(CHECK_WIDTH + NUMBER_WIDTH + 90.0, 86.0);
    run(&ctx, &mut state, &[], vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        run(
            &ctx,
            &mut state,
            &[],
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            }],
        );
    }
    assert_eq!(state.focus, Some(Cell { row: 0, column: 0 }));
    let (_, output) = run(
        &ctx,
        &mut state,
        &[],
        vec![egui::Event::Key {
            key: Key::ArrowRight,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }],
    );
    assert_eq!(output.toggle_tree, Some(RowId(1)));
    assert_eq!(state.focus, Some(Cell { row: 0, column: 0 }));
}
