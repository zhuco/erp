//! Reusable condition editor. Hosts own query dispatch and debounce.
use egui::{Id, Ui};
use erp_grid_core::{
    Column, ColumnId, Filter, FilterDraft, FilterKind,
    paged_query::{FilterExpression, civil_parts},
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Node {
    Rule(ColumnId, FilterDraft),
    Group { any: bool, children: Vec<Node> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryBuilder {
    root: Node,
}
impl Default for QueryBuilder {
    fn default() -> Self {
        Self {
            root: Node::Group {
                any: false,
                children: Vec::new(),
            },
        }
    }
}
impl QueryBuilder {
    pub fn from_expression(
        expression: Option<&FilterExpression>,
        columns: &[Column],
    ) -> Result<Self, String> {
        let Some(expression) = expression else {
            return Ok(Self::default());
        };
        expression.validate().map_err(str::to_owned)?;

        fn money_text(value: i64) -> String {
            let magnitude = value.unsigned_abs();
            let sign = if value < 0 { "-" } else { "" };
            format!("{sign}{}.{:02}", magnitude / 100, magnitude % 100)
        }

        fn restore(node: &FilterExpression, columns: &[Column]) -> Result<Node, String> {
            match node {
                FilterExpression::Condition { column, filter } => {
                    let definition = columns
                        .iter()
                        .find(|candidate| candidate.id == *column)
                        .ok_or_else(|| format!("Unknown condition column {}", column.0))?;
                    let draft = match (filter, &definition.filter) {
                        (Filter::Text(text), FilterKind::Text | FilterKind::Enum(_)) => {
                            FilterDraft::Text(text.clone())
                        }
                        (Filter::Money { min, max }, FilterKind::Money) => FilterDraft::Money {
                            min: min.map(money_text).unwrap_or_default(),
                            max: max.map(money_text).unwrap_or_default(),
                        },
                        (Filter::DateTime { start, end }, FilterKind::DateTime) => {
                            FilterDraft::DateTime {
                                start: start.map(civil_parts).transpose()?,
                                end: end.map(civil_parts).transpose()?,
                            }
                        }
                        (Filter::Enum(values), FilterKind::Text | FilterKind::Enum(_)) => {
                            FilterDraft::Enum(values.clone())
                        }
                        _ => {
                            return Err(format!(
                                "Saved condition does not match column {}",
                                column.0
                            ));
                        }
                    };
                    Ok(Node::Rule(*column, draft))
                }
                FilterExpression::And(children) | FilterExpression::Or(children) => {
                    let children = children
                        .iter()
                        .map(|child| restore(child, columns))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(Node::Group {
                        any: matches!(node, FilterExpression::Or(_)),
                        children,
                    })
                }
            }
        }

        let candidate = Self {
            root: restore(expression, columns)?,
        };
        let compiled = candidate.compile(columns)?;
        if compiled.as_ref() != Some(expression) {
            return Err("Saved condition cannot be restored without changing its meaning".into());
        }
        Ok(candidate)
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn compile(&self, columns: &[Column]) -> Result<Option<FilterExpression>, String> {
        fn compile(
            node: &Node,
            columns: &[Column],
            root: bool,
        ) -> Result<Option<FilterExpression>, String> {
            match node {
                Node::Rule(id, draft) => {
                    let column = columns
                        .iter()
                        .find(|column| column.id == *id)
                        .ok_or("Unknown condition column")?;
                    // Keep text conditions as substring matches if an option directory arrives later.
                    let kind = match draft {
                        FilterDraft::Text(_) => erp_grid_core::FilterKind::Text,
                        FilterDraft::Enum(selected) => {
                            erp_grid_core::FilterKind::Enum(selected.iter().cloned().collect())
                        }
                        _ => column.filter.clone(),
                    };
                    let filter = draft
                        .compile(&kind)
                        .map_err(str::to_owned)?
                        .ok_or("Condition needs a value")?;
                    Ok(Some(FilterExpression::Condition {
                        column: *id,
                        filter,
                    }))
                }
                Node::Group { any, children } => {
                    if children.is_empty() {
                        return if root {
                            Ok(None)
                        } else {
                            Err("Condition group is empty".into())
                        };
                    }
                    let mut expressions = Vec::with_capacity(children.len());
                    for child in children {
                        if let Some(expression) = compile(child, columns, false)? {
                            expressions.push(expression);
                        }
                    }
                    Ok(Some(if *any {
                        FilterExpression::Or(expressions)
                    } else {
                        FilterExpression::And(expressions)
                    }))
                }
            }
        }
        compile(&self.root, columns, true)
    }

    pub fn show(&mut self, ui: &mut Ui, id: Id, columns: &[Column]) -> bool {
        let before = self.clone();
        fn count(node: &Node) -> usize {
            match node {
                Node::Rule(..) => 1,
                Node::Group { children, .. } => 1 + children.iter().map(count).sum::<usize>(),
            }
        }
        let mut remaining = 64usize.saturating_sub(count(&self.root));
        fn show(
            node: &mut Node,
            ui: &mut Ui,
            id: Id,
            columns: &[Column],
            depth: usize,
            remaining: &mut usize,
        ) {
            ui.push_id(id, |ui| match node {
                Node::Rule(column_id, draft) => {
                    ui.horizontal(|ui| {
                        let old = *column_id;
                        egui::ComboBox::from_id_salt("column")
                            .selected_text(
                                columns
                                    .iter()
                                    .find(|c| c.id == *column_id)
                                    .map_or("Column", |c| c.title.as_str()),
                            )
                            .show_ui(ui, |ui| {
                                for column in columns
                                    .iter()
                                    .filter(|column| !matches!(column.filter, FilterKind::None))
                                {
                                    ui.selectable_value(column_id, column.id, &column.title);
                                }
                            });
                        if let Some(column) = columns.iter().find(|c| c.id == *column_id) {
                            if old != *column_id {
                                *draft = FilterDraft::for_kind(&column.filter);
                            }
                            let mut column = column.clone();
                            if matches!(draft, FilterDraft::Text(_)) {
                                column.filter = erp_grid_core::FilterKind::Text;
                            }
                            let (_, rect) = ui.allocate_space(egui::vec2(240.0, 24.0));
                            crate::quick_search::show(ui, rect, id.with("value"), &column, draft);
                        }
                    });
                }
                Node::Group { any, children } => {
                    ui.horizontal(|ui| {
                        ui.selectable_value(any, false, "AND");
                        ui.selectable_value(any, true, "OR");
                        if ui
                            .add_enabled(
                                *remaining > 0
                                    && columns
                                        .iter()
                                        .any(|column| !matches!(column.filter, FilterKind::None)),
                                egui::Button::new("+ Condition"),
                            )
                            .clicked()
                            && let Some(column) = columns
                                .iter()
                                .find(|column| !matches!(column.filter, FilterKind::None))
                        {
                            children
                                .push(Node::Rule(column.id, FilterDraft::for_kind(&column.filter)));
                            *remaining -= 1;
                        }
                        if ui
                            .add_enabled(*remaining > 0 && depth < 6, egui::Button::new("+ Group"))
                            .clicked()
                        {
                            children.push(Node::Group {
                                any: false,
                                children: Vec::new(),
                            });
                            *remaining -= 1;
                        }
                    });
                    let mut remove = None;
                    ui.indent("children", |ui| {
                        for (index, child) in children.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                if ui
                                    .small_button("×")
                                    .on_hover_text("Remove condition or group")
                                    .clicked()
                                {
                                    remove = Some(index);
                                }
                                ui.vertical(|ui| {
                                    show(child, ui, id.with(index), columns, depth + 1, remaining)
                                });
                            });
                        }
                    });
                    if let Some(index) = remove {
                        children.remove(index);
                    }
                }
            });
        }
        show(&mut self.root, ui, id, columns, 0, &mut remaining);
        if let Err(error) = self.compile(columns) {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        *self != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn query_builder_preserves_nested_semantics_and_rejects_incomplete_rules() {
        let columns = [Column::new(ColumnId(0), "Name", 120.0)];
        let mut builder = QueryBuilder::default();
        assert_eq!(builder.compile(&columns), Ok(None));
        builder.root = Node::Group {
            any: false,
            children: vec![Node::Group {
                any: true,
                children: vec![
                    Node::Rule(ColumnId(0), FilterDraft::Text("A".into())),
                    Node::Rule(ColumnId(0), FilterDraft::Text("B".into())),
                ],
            }],
        };
        let expression = builder.compile(&columns).unwrap().unwrap();
        assert!(
            matches!(expression, FilterExpression::And(items) if matches!(&items[0], FilterExpression::Or(inner) if inner.len() == 2))
        );
        builder.root = Node::Rule(ColumnId(0), FilterDraft::Text(String::new()));
        assert!(builder.compile(&columns).is_err());
    }

    #[test]
    fn saved_expression_round_trips_nested_money_and_utc_dates_exactly() {
        let columns = [
            Column::new(ColumnId(1), "Amount", 120.0).with_filter(FilterKind::Money),
            Column::new(ColumnId(2), "Created", 160.0).with_filter(FilterKind::DateTime),
            Column::new(ColumnId(3), "Status", 100.0)
                .with_filter(FilterKind::Enum(vec!["paid".into(), "open".into()])),
        ];
        let expression = FilterExpression::And(vec![
            FilterExpression::Or(vec![
                FilterExpression::Condition {
                    column: ColumnId(1),
                    filter: Filter::Money {
                        min: Some(-5),
                        max: Some(1_234_567),
                    },
                },
                FilterExpression::Condition {
                    column: ColumnId(2),
                    filter: Filter::DateTime {
                        start: Some(20_240_229_123_456),
                        end: Some(20_251_231_235_959),
                    },
                },
            ]),
            FilterExpression::Condition {
                column: ColumnId(3),
                filter: Filter::Enum(BTreeSet::from(["paid".into()])),
            },
        ]);

        let restored = QueryBuilder::from_expression(Some(&expression), &columns).unwrap();
        assert_eq!(restored.compile(&columns), Ok(Some(expression)));
        assert_eq!(
            QueryBuilder::from_expression(None, &columns),
            Ok(QueryBuilder::default())
        );
    }

    #[test]
    fn saved_expression_rejects_malformed_groups_and_invalid_columns() {
        let columns = [Column::new(ColumnId(1), "Name", 120.0)];
        assert!(
            QueryBuilder::from_expression(Some(&FilterExpression::And(Vec::new())), &columns)
                .is_err()
        );

        let unknown = FilterExpression::Condition {
            column: ColumnId(99),
            filter: Filter::Text("missing".into()),
        };
        assert!(QueryBuilder::from_expression(Some(&unknown), &columns).is_err());
    }

    #[test]
    fn saved_text_and_enum_filters_round_trip_on_a_text_column() {
        let columns = [Column::new(ColumnId(1), "Status", 120.0)];
        let expression = FilterExpression::And(vec![
            FilterExpression::Condition {
                column: ColumnId(1),
                filter: Filter::Text("paid".into()),
            },
            FilterExpression::Condition {
                column: ColumnId(1),
                filter: Filter::Enum(BTreeSet::from(["archived".into()])),
            },
        ]);

        let restored = QueryBuilder::from_expression(Some(&expression), &columns).unwrap();
        assert_eq!(restored.compile(&columns), Ok(Some(expression)));
    }
}
