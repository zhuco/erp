use super::*;

fn frame(
    ctx: &egui::Context,
    state: &mut GridState,
    columns: &mut Vec<Column>,
    events: Vec<egui::Event>,
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
                    id: Id::new("summary-test"),
                    state,
                    columns,
                    rows: 100,
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

fn position(text: &[(String, Pos2)], needle: &str) -> Pos2 {
    text.iter()
        .find(|(value, _)| value == needle)
        .unwrap_or_else(|| panic!("missing {needle}: {text:?}"))
        .1
}

#[test]
fn summary_is_fixed_vertically_and_tracks_locked_and_scrolling_columns() {
    let ctx = egui::Context::default();
    ctx.style_mut_of(egui::Theme::Dark, |style| style.animation_time = 0.0);
    ctx.style_mut_of(egui::Theme::Light, |style| style.animation_time = 0.0);
    let mut state = GridState {
        summary_rows: vec![SummaryRow {
            label: "合计".into(),
            cells: [
                (ColumnId(1), "locked-total".into()),
                (ColumnId(2), "moving-total".into()),
            ]
            .into_iter()
            .collect(),
        }],
        ..Default::default()
    };
    let mut locked = Column::new(ColumnId(1), "Locked", 160.0);
    locked.pinned = true;
    let mut columns = vec![locked, Column::new(ColumnId(2), "Moving", 900.0)];
    for _ in 0..2 {
        frame(&ctx, &mut state, &mut columns, vec![]);
    }
    let initial = frame(&ctx, &mut state, &mut columns, vec![]);
    let locked = position(&initial, "locked-total");
    let moving = position(&initial, "moving-total");
    let label = position(&initial, "合计");
    state.scroll = Vec2::new(180.0, 420.0);
    for _ in 0..2 {
        frame(&ctx, &mut state, &mut columns, vec![]);
    }
    let scrolled = frame(&ctx, &mut state, &mut columns, vec![]);
    assert_eq!(position(&scrolled, "locked-total"), locked);
    assert_eq!(position(&scrolled, "locked-total").y, label.y);
    assert_eq!(position(&scrolled, "moving-total").y, moving.y);
    assert!(position(&scrolled, "moving-total").x < moving.x);
}

#[test]
fn summary_cells_do_not_select_activate_or_edit_rows() {
    let ctx = egui::Context::default();
    let mut state = GridState {
        summary_rows: vec![SummaryRow {
            label: "Total".into(),
            cells: [(ColumnId(1), "summary-click-target".into())]
                .into_iter()
                .collect(),
        }],
        ..Default::default()
    };
    let mut columns = vec![Column::new(ColumnId(1), "Value", 700.0)];
    let text = frame(&ctx, &mut state, &mut columns, vec![]);
    let pos = position(&text, "summary-click-target") + Vec2::new(4.0, 4.0);
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
    assert!(state.selection.regions().is_empty());
    assert!(state.row_selection.checked().is_empty());
    assert_eq!(state.row_selection.active, None);
    assert!(!state.is_editing());
}

#[test]
fn summary_limits_rows_and_utf8_values_with_feedback() {
    let ctx = egui::Context::default();
    let long = "界".repeat(2_000);
    let mut state = GridState {
        summary_rows: (1..=4)
            .map(|index| SummaryRow {
                label: format!("total-{index}"),
                cells: [(
                    ColumnId(1),
                    if index == 1 {
                        long.clone()
                    } else {
                        format!("v{index}")
                    },
                )]
                .into_iter()
                .collect(),
            })
            .collect(),
        ..Default::default()
    };
    let mut columns = vec![Column::new(ColumnId(1), "Value", 700.0)];
    let text = frame(&ctx, &mut state, &mut columns, vec![]);
    assert!(text.iter().any(|(value, _)| value == "total-3"));
    assert!(!text.iter().any(|(value, _)| value == "total-4"));
    let rendered = text
        .iter()
        .find(|(value, _)| value.starts_with('界'))
        .expect("bounded UTF-8 summary value must be painted");
    assert!(rendered.0.len() <= MAX_SUMMARY_VALUE_BYTES);
    assert!(
        state
            .message
            .as_deref()
            .is_some_and(|message| message.contains("4096"))
    );
}
