use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TreeNode {
    pub id: String,
    pub label: String,
    pub count: Option<usize>,
    pub children: Vec<TreeNode>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TreeFilter {
    pub expanded: BTreeSet<String>,
    pub selected: Option<String>,
}

impl TreeFilter {
    /// Draws the tree and returns a newly selected node id. Counts are supplied
    /// by the host and are never fetched or inferred here.
    pub fn show(&mut self, ui: &mut egui::Ui, nodes: &[TreeNode]) -> Option<String> {
        let mut selected = None;
        let tree_id = ui.id().with("tree-filter");
        let flat = visible_nodes(nodes, &self.expanded);
        let has_focus = flat.iter().any(|(node, _)| {
            ui.memory(|memory| memory.has_focus(tree_id.with(("focus", node.id.as_str()))))
        });
        if has_focus {
            ui.input_mut(|input| {
                let events = input.events.clone();
                for event in &events {
                    let egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } = event
                    else {
                        continue;
                    };
                    // Reserve modified arrows for the host/application.  Without
                    // this guard Ctrl+Arrow could move the tree selection while
                    // the host is handling a navigation shortcut.
                    if *modifiers != egui::Modifiers::NONE && *key != egui::Key::Enter {
                        continue;
                    }
                    let Some(current) = self
                        .selected
                        .as_ref()
                        .and_then(|id| flat.iter().position(|(node, _)| node.id == *id))
                    else {
                        continue;
                    };
                    let next = match key {
                        egui::Key::ArrowDown => {
                            Some((current + 1).min(flat.len().saturating_sub(1)))
                        }
                        egui::Key::ArrowUp => Some(current.saturating_sub(1)),
                        egui::Key::ArrowRight => {
                            let node = flat[current].0;
                            (!node.children.is_empty()).then(|| {
                                self.expanded.insert(node.id.clone());
                                current
                            })
                        }
                        egui::Key::ArrowLeft => {
                            let node = flat[current].0;
                            if self.expanded.remove(&node.id) {
                                Some(current)
                            } else {
                                parent_index(&flat, current)
                            }
                        }
                        egui::Key::Enter => Some(current),
                        _ => None,
                    };
                    if let Some(index) = next {
                        let id = flat[index].0.id.clone();
                        if id != self.selected.as_deref().unwrap_or_default()
                            || matches!(key, egui::Key::Enter)
                        {
                            self.selected = Some(id.clone());
                            selected = Some(id);
                        }
                        input.consume_key(egui::Modifiers::NONE, *key);
                    }
                }
            });
        }
        for (node, depth) in flat {
            let row_id = tree_id.with(&node.id);
            let response = ui
                .scope_builder(egui::UiBuilder::new().id_salt(row_id), |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(depth as f32 * 16.0);
                        if !node.children.is_empty() {
                            let open = self.expanded.contains(&node.id);
                            if ui
                                .add(egui::Button::new(if open { "-" } else { "+" }).frame(false))
                                .clicked()
                            {
                                if open {
                                    self.expanded.remove(&node.id);
                                } else {
                                    self.expanded.insert(node.id.clone());
                                }
                            }
                        } else {
                            ui.add_space(18.0);
                        }
                        let selected = self.selected.as_deref() == Some(node.id.as_str());
                        ui.add_sized(
                            [ui.available_width(), 26.0],
                            egui::Button::new("")
                                .left_text(&node.label)
                                .right_text(
                                    egui::RichText::new(
                                        node.count.map(|n| n.to_string()).unwrap_or_default(),
                                    )
                                    .color(ui.visuals().weak_text_color()),
                                )
                                .selected(selected)
                                .frame(selected),
                        )
                    })
                    .inner
                })
                .inner;
            let focus_id = tree_id.with(("focus", node.id.as_str()));
            let focus_response = ui.interact(response.rect, focus_id, egui::Sense::click());
            if response.clicked() || focus_response.clicked() {
                focus_response.request_focus();
                self.selected = Some(node.id.clone());
                selected = Some(node.id.clone());
            }
        }
        selected
    }
}

fn visible_nodes<'a>(
    nodes: &'a [TreeNode],
    expanded: &BTreeSet<String>,
) -> Vec<(&'a TreeNode, usize)> {
    fn visit<'a>(
        nodes: &'a [TreeNode],
        depth: usize,
        expanded: &BTreeSet<String>,
        out: &mut Vec<(&'a TreeNode, usize)>,
    ) {
        for node in nodes {
            out.push((node, depth));
            if expanded.contains(&node.id) {
                visit(&node.children, depth + 1, expanded, out);
            }
        }
    }
    let mut out = Vec::new();
    visit(nodes, 0, expanded, &mut out);
    out
}

fn parent_index(flat: &[(&TreeNode, usize)], current: usize) -> Option<usize> {
    let depth = flat[current].1;
    (0..current).rev().find(|index| flat[*index].1 + 1 == depth)
}
