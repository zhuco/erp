use super::*;
use erp_grid_core::actions::ActionSpec;

fn frame(
    ctx: &egui::Context,
    state: &mut GridState,
    columns: &mut Vec<Column>,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, Option<(String, Vec<RowId>)>) {
    let mut action = None;
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            events,
            ..Default::default()
        },
        |root| {
            egui::CentralPanel::default().show(root, |ui| {
                action = DataGrid {
                    id: Id::new("business-actions"),
                    columns,
                    state,
                    rows: 10,
                    row_id: Some(&|row| RowId(row as u128 + 100)),
                }
                .show(ui, |_, _| "Product".into())
                .row_action;
            });
        },
    );
    output.textures_delta.clear();
    (output, action)
}
fn click(
    ctx: &egui::Context,
    state: &mut GridState,
    columns: &mut Vec<Column>,
    pos: Pos2,
    button: egui::PointerButton,
) -> (egui::FullOutput, Option<(String, Vec<RowId>)>) {
    frame(ctx, state, columns, vec![egui::Event::PointerMoved(pos)]);
    frame(
        ctx,
        state,
        columns,
        vec![egui::Event::PointerButton {
            pos,
            button,
            pressed: true,
            modifiers: Default::default(),
        }],
    );
    let (_, action) = frame(
        ctx,
        state,
        columns,
        vec![egui::Event::PointerButton {
            pos,
            button,
            pressed: false,
            modifiers: Default::default(),
        }],
    );
    (frame(ctx, state, columns, vec![]).0, action)
}
fn action_position(output: &egui::FullOutput) -> Pos2 {
    fn find(shape: &egui::epaint::Shape) -> Option<Pos2> {
        match shape {
            egui::epaint::Shape::Text(text) if text.galley.text() == "Business copy" => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            egui::epaint::Shape::Vec(shapes) => shapes.iter().find_map(find),
            _ => None,
        }
    }
    output
        .shapes
        .iter()
        .find_map(|shape| find(&shape.shape))
        .expect("business action menu must be painted")
}

#[test]
fn action_menu_enforces_target_limits_and_emits_frozen_checked_ids() {
    let ctx = egui::Context::default();
    let mut state = GridState {
        context_actions: vec![ActionSpec::new("products.copy", "Business copy", 1)],
        ..Default::default()
    };
    let mut columns = vec![Column::new(ColumnId(1), "Name", 250.0)];
    frame(&ctx, &mut state, &mut columns, vec![]);
    state.row_selection.toggle(RowId(1));
    state.row_selection.toggle(RowId(2));
    let (output, _) = click(
        &ctx,
        &mut state,
        &mut columns,
        Pos2::new(160.0, 100.0),
        egui::PointerButton::Secondary,
    );
    let pos = action_position(&output);
    let (_, action) = click(
        &ctx,
        &mut state,
        &mut columns,
        pos,
        egui::PointerButton::Primary,
    );
    assert!(
        action.is_none(),
        "single-target action cannot silently choose one checked row"
    );
    state.context_actions[0].max_targets = 10;
    click(
        &ctx,
        &mut state,
        &mut columns,
        Pos2::new(600.0, 30.0),
        egui::PointerButton::Primary,
    );
    let (output, _) = click(
        &ctx,
        &mut state,
        &mut columns,
        Pos2::new(160.0, 100.0),
        egui::PointerButton::Secondary,
    );
    let pos = action_position(&output);
    let (_, action) = click(
        &ctx,
        &mut state,
        &mut columns,
        pos,
        egui::PointerButton::Primary,
    );
    state.row_selection.clear();
    assert_eq!(
        action,
        Some(("products.copy".into(), vec![RowId(1), RowId(2)]))
    );
}
